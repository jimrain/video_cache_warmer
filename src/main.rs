//! Video Cache Warmer at the ege. This app works on HLS only. Once it reads a playlist manifest
//! it makes calls for the first X video segments so that they will be in cache.

// use fastly::http::request::{PendingRequest, SendError};
use fastly::http::{header, Method, StatusCode};
use fastly::{Error, Request, Response};
use lazy_static::lazy_static;
use m3u8_rs::playlist::{MediaPlaylist, Playlist};
use regex::Regex;
use std::str;

/// The name of a backend server associated with this service.
///
/// This backend is defined in my service but you can change it to whatever backend you want,
/// but it should have an m3u8 somewhere in it.
const BACKEND: &str = "ShastaRain_backend";

/// The number os segments to pre load - increase/decrease this as you see fit.
const NUM_SEGMENTS_TO_PRELOAD: usize = 5;

/// The entry point for your application.
///
/// This function is triggered when your service receives a client request. It could be used to
/// route based on the request properties (such as method or path), send the request to a backend,
/// make completely new requests, and/or generate synthetic responses.
///
/// If `main` returns an error, a 500 error response will be delivered to the client.
#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    // Make sure we are running the version we think we are.

    println!(
        "Video Cache Warmer version:{}",
        std::env::var("FASTLY_SERVICE_VERSION").unwrap_or_else(|_| String::new())
    );

    // Filter request methods...
    match req.get_method() {
        // Allow GET and HEAD requests.
        &Method::GET | &Method::HEAD => (),

        // Accept PURGE requests; it does not matter to which backend they are sent.
        m if m == "PURGE" => return Ok(req.send(BACKEND)?),

        // Deny anything else.
        _ => {
            return Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED)
                .with_header(header::ALLOW, "GET, HEAD")
                .with_body_text_plain("This method is not allowed\n"))
        }
    };

    // req.get_header_str() = None;

    // If this is an m3u8 file parse it, other wise let it fall through to the backend.
    if req.get_path().ends_with(".m3u8") && req.get_method() == Method::GET {
        let path_str = req.get_path().to_owned();
        let req_url = req.get_url_str().to_owned();
        println!("URL: {}", req.get_url_str());
        let mut beresp = req.send(BACKEND)?;

        let mut new_resp = beresp.clone_with_body();
        // let mut body_bytes = new_resp.take_body_bytes();
        match m3u8_rs::parse_playlist_res(new_resp.take_body_bytes().as_slice()) {
            Ok(Playlist::MasterPlaylist(_pl)) => println!("Master playlist"),
            Ok(Playlist::MediaPlaylist(pl)) => {
                println!("Media Playlist. Path = {}", path_str);
                send_media_segments_requests_async(&pl, req_url)?;
            }
            Err(_e) => fastly::error::bail!("Invalid manifest"),
        }
        // I got what I needed so return the beresp in a Result
        Ok(beresp)
    } else {
        Ok(req.send(BACKEND)?)
    }
}

/// This function takes a media playlist and makes async calls to the backend for the first N
/// media segments. It then does a select on the pending responses and logs the headers as the
/// responses are returned.
/// Note: In a real world scenario we wouldn't log the responses, we would just make the calls then
/// pass the manifest back up to the client. Then it would be blazingly fast.
fn send_media_segments_requests_async(
    playlist: &MediaPlaylist,
    req_url: String,
) -> Result<(), fastly::Error> {
    let mut pending_reqs = vec![];
    let req_uri_without_filename = get_path_to_m3u8(req_url.as_str());
    for (_index, segment) in playlist
        .segments
        .iter()
        .enumerate()
        .filter(|&(index, _)| index < NUM_SEGMENTS_TO_PRELOAD)
    {
        // Build the abosolute path for the ts segment then send a head request to that url.
        let url = format!("{}{}",  req_uri_without_filename, segment.uri);
        pending_reqs.push(Request::head(url).send_async(BACKEND));
    }

    /* This loop is a debug step only to make sure you are getting the correct responses back from
        the origin. Leave it commented out for production as it will slow down the response
        going back to the client.
    while !pending_reqs.is_empty() {
        let (resp, new_pending_reqs) = fastly::http::request::select(pending_reqs);
        let respu = resp.unwrap();
        println!("Recieved response from backend. Status: {} X-Cache: {} X-Served-by: {}",
                 respu.get_status().as_str(), respu.get_header_str("X-Cache").unwrap(),
        respu.get_header_str("X-served-by").unwrap());
        pending_reqs = new_pending_reqs;
    }
     */

    Ok(())
}


/// Special case regex function to get just the path to the m3u8 file witout the file name or
/// extension.
fn get_path_to_m3u8(input: &str) -> std::string::String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^(?P<path>.*/).*.m3u8/*$").unwrap();
    }
    let capture = RE
        .captures(input)
        .and_then(|cap| cap.name("path").map(|path| path.as_str()));
    capture.as_deref().unwrap_or("").to_string()
}

