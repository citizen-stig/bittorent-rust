
 ~ Bencode decoding: learning serde::deserializer
  + int
  + string
  + list
  + dict
  - Try `serde_json` with vetted values to verify
  - Map only string keys
  + Proper struct deserialization
  - json based mixed values
 ~ Parsing torrent files
  - Change how strings are parsed: convert base parsing to be bytes, and then string attempts from it!
 - Peer 2 peer communication
 + Piece downloading
 + Full file download

# Bencode step pack

 - Implement naive parsing without serde
   - all types
   - error cases
   - iterators
   - zero copy


Questions
 - How serde suppose to handle deserialize_string when only deserialize_str is implemented?
 - Bencode: should it allow to deserialize same encoding into different types, like integers? Or should it be strict as hell?