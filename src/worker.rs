//! One-shot parent/worker execution over bounded PHIP streams.
//!
//! This is process separation only. It does not install OS sandboxing,
//! resource quotas, or capability enforcement.

use std::{
    io::{Read, Write},
    process::{Command, Stdio},
};

use crate::security::ipc::{
    read_frame, write_frame, EndpointRole, Frame, Message, Session, PROTOCOL_VERSION,
};

const REQUEST_ID: u64 = 1;
const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_CAPTURED_OUTPUT: usize = 1024 * 1024;

pub fn run_isolated_source(source: &str) -> Result<Vec<String>, String> {
    if source.is_empty() || source.len() > MAX_SOURCE_BYTES {
        return Err("Source is empty or exceeds the worker input limit".to_owned());
    }
    let executable = std::env::current_exe()
        .map_err(|error| format!("Cannot locate Phoneal executable: {error}"))?;
    let mut child = Command::new(executable)
        .arg("__worker")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Cannot start Phoneal worker: {error}"))?;

    let result = communicate_with_worker(&mut child, source);
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait()
        .map_err(|error| format!("Cannot wait for Phoneal worker: {error}"))?;
    match result {
        Ok(output) if status.success() => Ok(output),
        Ok(_) => Err(format!("Phoneal worker exited with status {status}")),
        Err(error) => Err(error),
    }
}

fn communicate_with_worker(child: &mut std::process::Child, source: &str) -> Result<Vec<String>, String> {
    let mut input = child.stdin.take().ok_or("Worker stdin was not available")?;
    let mut output = child.stdout.take().ok_or("Worker stdout was not available")?;
    let mut parent = Session::new(EndpointRole::Parent);

    let hello = Frame::from_message(0, &Message::Hello {
        minimum_version: PROTOCOL_VERSION,
        maximum_version: PROTOCOL_VERSION,
    }).map_err(|error| error.to_string())?;
    write_frame(&mut input, &hello).map_err(|error| error.to_string())?;
    let response = read_frame(&mut output)
        .map_err(|error| error.to_string())?
        .ok_or("Worker closed IPC before its handshake")?;
    match parent.accept(&response).map_err(|error| error.to_string())? {
        Message::Hello { .. } => {}
        _ => return Err("Worker sent an unexpected handshake response".to_owned()),
    }

    parent.begin_request(REQUEST_ID).map_err(|error| error.to_string())?;
    let request = Frame::from_message(REQUEST_ID, &Message::RunSource(source.to_owned()))
        .map_err(|error| error.to_string())?;
    write_frame(&mut input, &request).map_err(|error| error.to_string())?;
    drop(input);

    let mut lines = Vec::new();
    let mut output_bytes = 0_usize;
    loop {
        let frame = read_frame(&mut output)
            .map_err(|error| error.to_string())?
            .ok_or("Worker closed IPC before completing the request")?;
        match parent.accept(&frame).map_err(|error| error.to_string())? {
            Message::Stdout(bytes) => {
                output_bytes = output_bytes.checked_add(bytes.len())
                    .filter(|size| *size <= MAX_CAPTURED_OUTPUT)
                    .ok_or("Worker output exceeds the capture limit")?;
                lines.push(String::from_utf8(bytes).map_err(|_| "Worker output is not valid UTF-8")?);
            }
            Message::Stderr(_) => return Err("Worker reported an unexpected stderr frame".to_owned()),
            Message::Completed => break,
            Message::Error { code, message } => return Err(format!("Worker error {code}: {message}")),
            _ => return Err("Worker sent an unexpected response message".to_owned()),
        }
    }
    Ok(lines)
}

pub fn serve_worker<R: Read, W: Write>(input: &mut R, output: &mut W) -> Result<(), String> {
    let mut session = Session::new(EndpointRole::Worker);
    let hello = read_frame(input)
        .map_err(|error| error.to_string())?
        .ok_or("Parent closed IPC before handshake")?;
    match session.accept(&hello).map_err(|error| error.to_string())? {
        Message::Hello { .. } => {}
        _ => return Err("Parent sent an unexpected handshake message".to_owned()),
    }
    let response = Frame::from_message(0, &Message::Hello {
        minimum_version: PROTOCOL_VERSION,
        maximum_version: PROTOCOL_VERSION,
    }).map_err(|error| error.to_string())?;
    write_frame(output, &response).map_err(|error| error.to_string())?;

    let request = read_frame(input)
        .map_err(|error| error.to_string())?
        .ok_or("Parent closed IPC before source request")?;
    let request_id = request.request_id;
    let source = match session.accept(&request).map_err(|error| error.to_string())? {
        Message::RunSource(source) => source,
        _ => return Err("Worker expected a source execution request".to_owned()),
    };
    if source.len() > MAX_SOURCE_BYTES {
        return send_worker_error(output, &mut session, request_id, "Source exceeds the worker input limit");
    }

    match crate::run_bytecode(&source) {
        Ok(lines) => {
            let mut total = 0_usize;
            for line in lines {
                total = total.checked_add(line.len())
                    .filter(|size| *size <= MAX_CAPTURED_OUTPUT)
                    .ok_or("Program output exceeds the worker capture limit")?;
                let frame = Frame::from_message(request_id, &Message::Stdout(line))
                    .map_err(|error| error.to_string())?;
                write_frame(output, &frame).map_err(|error| error.to_string())?;
            }
            let completed = Frame::from_message(request_id, &Message::Completed)
                .map_err(|error| error.to_string())?;
            write_frame(output, &completed).map_err(|error| error.to_string())?;
            session.complete_request(request_id).map_err(|error| error.to_string())?;
            Ok(())
        }
        Err(error) => send_worker_error(output, &mut session, request_id, &error.render(&source)),
    }
}

fn send_worker_error<W: Write>(
    output: &mut W,
    session: &mut Session,
    request_id: u64,
    message: &str,
) -> Result<(), String> {
    let bounded = if message.len() > 16 * 1024 {
        let end = message.char_indices()
            .take_while(|(index, _)| *index <= 16 * 1024)
            .last()
            .map_or(0, |(index, character)| index + character.len_utf8());
        &message[..end]
    } else {
        message
    };
    let frame = Frame::from_message(request_id, &Message::Error {
        code: 1,
        message: bounded.to_owned(),
    }).map_err(|error| error.to_string())?;
    write_frame(output, &frame).map_err(|error| error.to_string())?;
    session.complete_request(request_id).map_err(|error| error.to_string())?;
    Ok(())
}
