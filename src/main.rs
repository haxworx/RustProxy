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

//   This is a HTTP proxy written in Rust!
//   DONE: POST+GET+OPTIONS+CONNECT
//   FIXED A STINKER OF A BUG!

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

    if srch.len() == 0 
    {
        return request;
    }    
    
    while buf[i] as char != '\0' 
    {
        if buf[i] as char == srch[0] as char 
        {
            while x < srch.len() && buf[i] as char  == srch[x] as char 
            {
                i = i + 1;
                x = x + 1;
            }
            break;
        } else {
            i = i + 1;
        }
    }

    if x == srch.len() 
    {    
        //println!("match");
        let mut end = i;
        while buf[end] as char != byte as char && buf[end] as char != '\n' 
        {
            end = end + 1;

        }

        for y in (i..end) 
        {    
            request.push(buf[y] as char);
        }
    }    

    return request;
} 

const CHUNK: usize = 4096; // PAGESIZEish

fn get_with_content_length(mut instream: &TcpStream, mut outstream: &TcpStream, total: usize) {
    let mut current = 0;
    let mut buf = [0u8; CHUNK];

    while current < total 
    {
        let bytes = outstream.read(&mut buf). unwrap();
        if bytes == 0 
        {
            break;    
        }

        let mut chunk = 0;

        while chunk < bytes 
        {
            let sent = instream.write(&mut buf[0..bytes]).unwrap();
            if sent <= 0 
            {
                break;
            }
            chunk = chunk + sent;
            current = current + chunk; 
        }
    }    
}

fn get_with_no_content_length(mut instream: &TcpStream, mut outstream: &TcpStream)
{
    loop {
        let mut buf = [0u8; CHUNK];
    
        let bytes = outstream.read(&mut buf).unwrap();
        if bytes == 0 
        {
            break; // please!!!
        }

        let mut chunk = 0;

        while chunk < bytes 
        {
            let sent = instream.write(&mut buf[0..bytes]).unwrap();
            if sent == 0 
            {
                break;
            }

            chunk = chunk + sent;
        }
    }
}

fn http_connect_request(instream: TcpStream, hdr: Header)
{
    let bogus_fix = &format!("{}:{}", hdr.hostname, 80);
    
    let outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();
    
    get_with_no_content_length(&instream, &outstream);    
}

fn http_options_request(mut instream: TcpStream)
{
    let allow = "GET,POST,HEAD,OPTIONS";
    let code: String = "HTTP/1.1 200 OK\r\n".to_string();
    let request: String  = "Allow: ".to_string() + allow + "\r\n\r\n";

    let response = format!("{}{}", code, request);

    instream.write(response.as_bytes()).unwrap();
}

fn http_head_request(instream: TcpStream, hdr: Header)
{
    let bogus_fix = &format!("{}:{}", hdr.hostname, 80);

    let mut outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();

    let query: String = format!("xHEAD /{} HTTP/1.1\r\nHost: {}\r\n\r\n", hdr.resource, hdr.hostname);

    outstream.write(query.as_bytes()).unwrap();
}

fn http_post_request(mut instream: TcpStream, headers: Header)
{
    let bogus_fix = &format!("{}:{}", headers.hostname, 80);
    let mut outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();    

    let query: String = format!("POST {}Host: {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n", headers.resource, headers.hostname, headers.content, headers.length);
    outstream.write(query.as_bytes()).unwrap();

    println!("QUERY: {}", query);

    let mut buf = [0u8; CHUNK];
    let mut current = 0;

    while current < headers.length
    {
        let bytes = instream.read(&mut buf).unwrap();
        if bytes == 0
        {
            break;
        }

        let mut chunk = 0;

        while chunk < bytes
        {
            let sent = outstream.write(&mut buf[0..bytes]).unwrap();
            if sent == 0
            {
                break;
            }
        
            chunk += sent;
            current = current + chunk;
        }
    }    

    let terminate = "\r\n\r\n";

    outstream.write(terminate.as_bytes()).unwrap();

    get_with_no_content_length(&instream, &outstream);
}

fn http_get_request(instream: TcpStream, headers: Header) 
{
    let bogus_fix = &format!("{}:{}", headers.hostname, 80);

    let mut outstream = TcpStream::connect::<(&str)>(bogus_fix).unwrap();

    let query: String = format!("GET {}Host: {}\r\nConnection: close\r\n\r\n", headers.resource, headers.hostname);
    outstream.write(query.as_bytes()).unwrap();

    println!("QUERY: {}", query);

    if headers.length > 0 
    {
        get_with_content_length(&instream, &outstream, headers.length);
    } 
    else 
    {
        get_with_no_content_length(&instream, &outstream);
    }
}

fn req_resource(buf: [u8;REQUEST_LEN]) -> String
{
    let mut i = 0;
    let bytes = buf;
    let mut request: String = String::new();
    
    while bytes[i] as char != '\0'
    {
        request.push(bytes[i] as char);
        i += 1;
    }
        
    return request;
}

fn req_content(buf: [u8;REQUEST_LEN]) -> String
{
    let content = substr(buf, "Content-Type: ", '\r' as u8);

    return content;
}

fn req_hostname(buf: [u8;REQUEST_LEN]) -> String
{
    let hostname: String = substr(buf, "http://", '/' as u8);    

    return hostname;
}

fn req_length(buf: [u8;REQUEST_LEN]) -> usize
{
    let request: String = substr(buf, "Content-Length: ", '\r' as u8);
    if ! request.is_empty()
    {
        return request.trim().parse().unwrap();
    }

    return 0;
}

struct Header {
    pub hostname: String,
    pub resource: String,    
    pub content: String,
    pub length: usize,
    pub method: String,
}

impl Header {

pub fn new() -> Header
{
    let hostname = String::new();
    let resource = String::new();
    let content  = String::new();
    let method   = String::new();
    let length   = 0;

    let h = Header
        { hostname: hostname, resource: resource , content: content, length: length, method: method};

    return h;
}

}


fn check_headers(buffer: [u8;REQUEST_LEN], headers: &mut Header) -> bool
{

    if headers.hostname.is_empty()
    {
        headers.hostname = req_hostname(buffer);
    }
    
    if headers.resource.is_empty()
    {
        headers.resource = req_resource(buffer);
    }

    if headers.content.is_empty()
    {
        headers.content = req_content(buffer);
    }

    if headers.length == 0
    {
        headers.length = req_length(buffer);
    }

    if headers.hostname.is_empty() || headers.resource.is_empty() || headers.method.is_empty() 
    {
        return false;
    }
    
    return true;
}

fn request_headers(mut instream: &TcpStream, headers: &mut Header) 
{
    let mut have_method = false;
    let mut byte = [0u8;1];
    let mut byte_count = 0;

    while ! have_method && byte_count < 16
    {
        instream.read(&mut byte).unwrap();
        if byte[0] as char != ' '
        {
            headers.method.push(byte[0] as char);
        }
        
        if byte[0] as char == ' '
        {
            have_method = true;
        }

        byte_count += 1;
    }

    if byte_count == 16 || headers.method.is_empty()
    {
        return;
    }

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
      
          buffer[len] = 0; 
  
          if check_headers(buffer, headers) && len == 2
          {
              return;
          }
     }
}


fn proxy(stream: TcpStream) {
    let mut headers: Header = Header::new();
    request_headers(&stream, &mut headers);

    match headers.method.as_ref() {
            "GET" => 
            {
                http_get_request(stream, headers);
            }

            "POST" =>
            {
                http_post_request(stream, headers);
            }

            "HEAD" =>
            {
                http_head_request(stream, headers);
            }

            "OPTIONS" =>
            {
                http_options_request(stream);
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
        
    proxy_time(port, threads);
    println!("Blocking all badness");

}

