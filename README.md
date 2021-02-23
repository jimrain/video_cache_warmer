# Compute@Edge Video Cache Warmer POC

This application warms the cache for a video asset by reading the playlist manifest (m3u8) and doing a head request
on the first N media segments. This pulls it into cache at the edge (and when shielding is supported it will cache
at the shield also. )


## Features

- Uses a 3rd party crate (m3u8_rs) to parse and manipulate hls manifests
- Uses async_send for head calls to make the response returned to the client quicker
- Match request URL path and methods for routing
- Build synthetic responses at the edge
- Optional troubleshooting loop that blocks on select for head request so you can print out responses. 
- Uses println! and eprintln! for log tailing

## Understanding the code

This code borrows from the boilerplate template to check whether the method is 'get', 'head', or 'purge'. If not it 
sends back a 405. 
After that check it looks to see if it's a 'get' and if the file extention is an m3u8. If so we use the m3u8_rs 
crate to parse the file and see if it's a master manifest or a playlist manifest. If it's a playlist manifest it 
reads the first N ts segments and sends an asynchrouns head request to each one. All other request (other then the 
playlist manifest) will just flow through to the backend. 

There is a debug loop that is commented out. If you want to see the responses from the head requests you can 
un comment this loop and start log tailing to see the output. This is a good way to prove that the ts segments are 
actually being cached. 

## Invocation
As is this can be tested out with this curl command:
curl https://weekly-flying-crab.edgecompute.app/videos/buck/buck_1080p.m3u8

then you can check that the segments where cached with this command:

curl -svo /dev/null https://weekly-flying-crab.edgecompute.app/videos/buck/buck_1080p_0002.ts -H "Fastly-Debug:1"

you should see x-cache: HIT

NOTE: if you see x-cache: MISS it means that the first curl and the second one hit to different pops. You can prove
this by uncommenting the trouble shooting loop and seeing which POP you hit with the head request. 

## Legal
This code is intended to be proof of concept code and is not maintained or supported by Fastly. 

## Security issues

Please see [SECURITY.md](SECURITY.md) for guidance on reporting security-related issues.
