/*
  Copyright (c) 2015, Al Poole <netstar@gmail.com>

  Permission to use, copy, modify, and/or distribute this software for any 
  purpose with or without fee is hereby granted, provided that the above 
  copyright notice and this permission notice appear in all copies.

  THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
  REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY 
  AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, 
  INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM 
  LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
  OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR 
  PERFORMANCE OF THIS SOFTWARE.
*/

//   This is a HTTPS CONNECT proxy!
//   it doesn't work yet as i don't know how to do this in Rust, is it 
//   even possible yet???

use std::string::{String};
use std::net::{SocketAddrV4,Ipv4Addr,TcpListener,TcpStream};
use std::io::{Write, Read};

const REQUEST_LEN: usize = 65535;

fn substr(buf: [u8;REQUEST_LEN], needle: &str, byte: u8) -> String
{
	let mut i = 0;
	let mut x = 0;
	let mut request = String::new();

	let srch = needle.as_bytes();

	if srch.len() == 0 {
		return request;
	}	
	
	while buf[i] as char != '\0'  {
		if buf[i] as char == srch[0] as char {
			while x < srch.len() && buf[i] as char  == srch[x] as char {
				i = i + 1;
				x = x + 1;
			}
			break;
		} else {
			i = i + 1;
		}
	}

	if x == srch.len() {
		let mut end = i;
		while buf[end] as char != byte as char && buf[end] as char != '\n' 
		{
			end = end + 1;

		}

		for y in i..end {
			request.push(buf[y] as char);
		}
	}	

	return request;
} 

const CHUNK: usize = 4096; // PAGESIZEish

fn talk_to_me(mut instream: &TcpStream, mut outstream: &TcpStream) -> bool
{

	let mut buf = [0u8; CHUNK];
	
	println!("top");
	let bytes = outstream.read(&mut buf).unwrap();
	if bytes <= 0 {
		return false;
	}

	println!("recv {}", bytes);

	for i in 0..bytes {
		print!("{}", buf[i] as char);
	}	
	   
	let mut chunk = 0;
   
	while chunk < bytes {
		let sent = instream.write(&mut buf[0..bytes]).unwrap();
		if sent <= 0 {
			return false;
		}
		println!("sent {}", sent);
		chunk = chunk + sent;
	}
	true
}

struct Header {
	pub hostname: String,
	pub connection: String,	
	pub proxy: String,
	pub agent: String,
}

impl Header {

pub fn new() -> Header
{
	let hostname	 = String::new();
	let connection   = String::new();
	let proxy	 = String::new();
	let agent	 = String::new();
 
	let h = Header
	{ 
		hostname: hostname,
		connection: connection,
		proxy: proxy,
		agent: agent
	};

	return h;
}

}

fn http_connect_request(instream: TcpStream, headers: Header)
{
	let bogus_fix = &format!("{}", headers.hostname);
		
	let outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();
	let mut disconnected = false;
	
	let response = format!("HTTP/1.1 200 Connection established\r\nUser-Agent: {}\r\nProxy-Connection: {}\r\nConnection: {}\r\nHost: {}\r\n\r\n",
			headers.agent, headers.proxy, headers.connection, headers.hostname);
	println!("sending: \n\n{}", response);
	let mut s = &instream;	
	s.write(response.as_bytes()).unwrap(); // tell client the connection is made

	// need to work out the best way to do this???
	// there is mio crate...
	// can we even do this in Rust yet???
	while ! disconnected {	
		disconnected = talk_to_me(&instream, &outstream);	
	}	
}

fn check_headers(buffer: [u8;REQUEST_LEN], headers: &mut Header) -> bool
{
	let mut i = 0;
	
	while buffer[i] != '\0' as u8
	{
		print!("{}", buffer[i] as char);
		i+=1;
	}
	print!("\n");

	if headers.hostname.is_empty()
	{
		headers.hostname = substr(buffer, "Host: ", '\r' as u8);
	}
	
	if headers.connection.is_empty()
	{
		headers.connection = substr(buffer, "Connection: ", '\r' as u8);
	}
	
	if headers.proxy.is_empty()
	{
		headers.proxy = substr(buffer, "Proxy-Connection: ", '\r' as u8);
	}
	
	if headers.agent.is_empty()
	{
		headers.agent = substr(buffer, "User-Agent: ", '\r' as u8);
	}
	
	if ! headers.hostname.is_empty() && ! headers.connection.is_empty() 
		&& ! headers.proxy.is_empty() && ! headers.agent.is_empty()
	{
		
		return true;
	}
	
	return false;
}

fn request_headers(mut instream: &TcpStream, headers: &mut Header) 
{
	 loop
	{
		let mut buffer = [0u8; REQUEST_LEN];
		let mut byte = [0u8; 1];
		let mut len = 0;
			
		while byte[0] as char != '\n'
		{
			let bytes = instream.read(&mut byte).unwrap();
			buffer[len] = byte[0];
			len += bytes;
		}
	  
		buffer[len] = '\0' as u8; 
  
		if check_headers(buffer, headers) && len == 2
		{
			let mut i = 0;
			
			while i < REQUEST_LEN
			{
				buffer[i] = 0;
				i += 1;
			}
			return;
		}
	}

}

fn request_method(mut instream: &TcpStream) -> String
{
	let mut byte = [0u8;1];

	let mut method: String = String::new();
	
	// bit better for now it'll do!
	while byte[0] as char != ' '
	{
		instream.read(&mut byte).unwrap();
		if byte[0] as char != ' '
		{
			method.push(byte[0] as char);
		}
	}
	
	match method.as_ref()
	{
		"CONNECT" =>
		{
			return method;
		}
	
		"GET" | "POST" | "OPTIONS" | "HEAD" =>
		{
			return "".to_string();
			// all good do not return!!!
		}
			
		_ =>
		{
			return "".to_string();
		}
	}
}


fn proxy(stream: TcpStream) {
	let method = request_method(&stream);

	match method.as_ref()
	{
		"CONNECT" =>
		{
			let mut headers = Header::new();
			request_headers(&stream, &mut headers);
	
			http_connect_request(stream, headers);
		}

		_ =>
		{
			println!("REQUEST UNKNOWN");
		}
	}	 
}

extern crate threadpool;
use threadpool::ThreadPool;

fn proxy_time(port: u16, threads: usize) { 
	let ip = Ipv4Addr::new(127, 0, 0, 1);
	let host = SocketAddrV4::new(ip, port);
	let listener = TcpListener::bind(host).unwrap();

	let pool = ThreadPool::new(threads);

	for stream in listener.incoming()
	{
		match stream
		{
			Ok(stream) => 
			{
				pool.execute(move || { proxy(stream); });	
			}

			Err(_) =>
			{
				println!("NET error");
				1 << 7;
			}
		}
	}	

	drop (listener);
}

fn main () {
	let threads = 128;
	let port: u16 = 9998;
		
	proxy_time(port, threads);
	println!("Blocking all badness");
}

