/*
    This is bogus but it's something to msss about with
*/

//   This is a HTTP proxy written in Rust!
//   DONE: POST+GET+OPTIONS

use std::string::{String};
use std::net::{SocketAddrV4,Ipv4Addr,TcpListener,TcpStream};
use std::io::{Write, Read};

const REQUEST_LEN: usize = 65535;

const CHUNK: usize = 4096; // PAGESIZEish

struct Request {
   pub headers: Header,
}

impl Request {

pub fn new() -> Request
{
        let headers = Header::new();

        let r = Request {
	   headers: headers,
        };

        r
}

fn get_with_content_length(self: &mut Request, mut instream: &TcpStream, mut outstream: &TcpStream) {
	let mut current = 0;
	let mut buf = [0u8; CHUNK];

	while current < self.headers.content_length {
		let bytes = outstream.read(&mut buf). unwrap();
		if bytes == 0 {
			break;	
		}

		let mut chunk = 0;

		while chunk < bytes {
			let sent = instream.write(&mut buf[0..bytes]).unwrap();
			if sent <= 0 {
				break;
			}
			chunk = chunk + sent;
			current = current + chunk; 
		}
	}	
}

fn get_with_no_content_length(self: &mut Request, mut instream: &TcpStream, mut outstream: &TcpStream)
{
	loop {
		let mut buf = [0u8; CHUNK];
	
		let bytes = outstream.read(&mut buf).unwrap();
		if bytes == 0 {
			break; // please!!!
		}

		let mut chunk = 0;

		while chunk < bytes {
			let sent = instream.write(&mut buf[0..bytes]).unwrap();
			if sent == 0 {
				break;
			}
			chunk = chunk + sent;
		}
	}
}

pub fn options(self: &mut Request, mut instream: TcpStream)
{
	let allow = "GET,POST,HEAD,CONNECT,OPTIONS";
	let code: String = "HTTP/1.1 200 OK\r\n".to_string();
	let request: String  = "Allow: ".to_string() + allow + "\r\n\r\n";

	let response = format!("{}{}", code, request);

	instream.write(response.as_bytes()).unwrap();
}

pub fn head(self: &mut Request)
{
	let bogus_fix = &format!("{}:{}", self.headers.hostname, 80);

	let mut outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();

	let query: String = format!("HEAD /{} HTTP/1.1\r\nHost: {}\r\n\r\n", self.headers.resource, self.headers.hostname);

	outstream.write(query.as_bytes()).unwrap();
}

pub fn post(self: &mut Request, mut instream: TcpStream)
{
	let bogus_fix = &format!("{}:{}", self.headers.hostname, 80);
	let mut outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();	

	let query: String = format!("POST {}Host: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n", self.headers.resource, self.headers.hostname, self.headers.content_type, self.headers.content_length);
	outstream.write(query.as_bytes()).unwrap();

	println!("QUERY: {}", query);

	let mut buf = [0u8; CHUNK];
	let mut current = 0;

	while current < self.headers.content_length {
		let bytes = instream.read(&mut buf).unwrap();
		if bytes == 0 {
			break;
		}

		let mut chunk = 0;

		while chunk < bytes {
			let sent = outstream.write(&mut buf[0..bytes]).unwrap();
			if sent == 0 {
				break;
			}
		
			chunk += sent;
			current = current + chunk;
		}
	}	

	let terminate = "\r\n\r\n";

	outstream.write(terminate.as_bytes()).unwrap();

	self.get_with_no_content_length(&instream, &outstream);
}

pub fn get(self: &mut Request, instream: TcpStream)
{

	let bogus_fix = &format!("{}:{}", self.headers.hostname, 80);

	let mut outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();

	let query: String = format!("GET {}Host: {}\r\nConnection: close\r\n\r\n", self.headers.resource, self.headers.hostname);
	outstream.write(query.as_bytes()).unwrap();

	println!("QUERY: {}", query);

	if self.headers.content_length > 0 {
		self.get_with_content_length(&instream, &outstream);
	} else { 
		self.get_with_no_content_length(&instream, &outstream);
	}
}

pub fn connect(self: &mut Request, instream: TcpStream)
{
	let bogus_fix = &format!("{}:{}", self.headers.hostname, 443);
	// this needs fixing!!!

        println!("here {}", self.headers.hostname);
	
	let outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();
	
	self.get_with_no_content_length(&instream, &outstream);	
}
}

struct Header {
	pub hostname: String,
	pub resource: String,	
	pub content_type: String,
	pub content_length: usize,
	pub method: String,
}

impl Header {

pub fn new() -> Header
{
	let hostname = String::new();
	let resource = String::new();
	let content_type  = String::new();
	let content_length = 0;
	let method = String::new();

	let h = Header {
		hostname: hostname, 
		resource: resource , 
		content_type: content_type, 
		content_length: content_length, 
		method: method
	};

	return h;
}

fn get(self: &mut Header, mut instream: &TcpStream)
{
	let mut byte = [0u8;1];

	// bit better for now it'll do!
	while byte[0] as char != ' ' {
		instream.read(&mut byte).unwrap();
		if byte[0] as char != ' '
		{
			self.method.push(byte[0] as char);
		}
	}
	
	match self.method.as_ref() {
		"GET" | "POST" | "OPTIONS" | "HEAD" | "CONNECT" =>
		{
			// all good do not return!!!
		}
			
		_ =>
		{
			return;
		}
	}
	
       loop {
		let mut buffer = [0u8; REQUEST_LEN];
		let mut byte = [0u8; 1];
		let mut len = 0;
			
		while byte[0] as char != '\n' {
			let bytes = instream.read(&mut byte).unwrap();
			buffer[len] = byte[0];
			len += bytes;
		}
	  
		buffer[len] = '\0' as u8; 

                if len == 2 && buffer[0] as char == '\r' && buffer[1] as char == '\n' {
                        return;
                }
 
	}
}

}

fn proxy(stream: TcpStream) {
        let mut http: Request = Request::new();
	http.headers.get(&stream);

	match http.headers.method.as_ref() {
		"GET" => 
		{
			http.get(stream);
		}

		"POST" =>
		{
			http.post(stream);
		}

		"HEAD" =>
		{
			http.head();
		}

		"OPTIONS" =>
		{
			http.options(stream);
		}  

		"CONNECT" =>
		{
			http.connect(stream);
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
	let port: u16 = 9999;
		
	println!("Blocking all badness on port {}", port);
	proxy_time(port, threads);
}

