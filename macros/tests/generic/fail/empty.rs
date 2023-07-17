//! Test that obviously invalid GUID's fails
use nuefi::GUID;

// Random UUID from `uuidgen` with 69420 added to it
const GUID: &str = "c986ec27-69420-af54-4b55-80aa-91697fcdf8eb";

#[GUID("")]
struct HasID;

#[GUID()]
struct HasID2;

#[GUID]
struct HasID3;

#[GUID(GUID)]
struct HasID4;

fn main() {}
