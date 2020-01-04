# vChain Demo

## Build

* Install Rust from <https://rustup.rs>.
* Run `cargo test` for unit test.
* Run `cargo build --release` to build the binaries, which will be located at `target/release/` folder.

## Create Block DB

### Input Format

The input is a text file with each line represent an object.

```
obj := block_id [ v_data ] { w_data }
v_data := v_1, v_2, ...
w_data := w_1, w_2, ...
```

For example

```
1 [1,2] {a,b,c}
1 [1,5] {a}
2 [3,4] {a,e}
```

### Build DB

Run `simchain-build` to build the database. You need to specify the bit length for each dimension of the v data. For example:

```
./target/release/simchain-build --bit-len 16,16 --skip-list-max-level 10 -i /path/to/data.txt -o /path/to/output_database
```

Run `simchain-build --help` for more info.

## Start the Server

Run `simchain-server` after the database is built. For example:

```
./target/release/simchain-server -b 127.0.0.1:8000 --db /path/to/database
```

Run `simchain-server --help` for more info.

## Server REST API

### Inspect

Use following API endpoint to inspect the blockchain. Returned response is a JSON object. Refer to source code for their definitions.

```
GET /get/param
GET /get/blk_header/{id}
GET /get/blk_data/{id}
GET /get/intraindex/{id}
GET /get/skiplist/{id}
GET /get/obj/{id}
```

### Query

API endpoint is:

```
POST /query
```

Encode query parameter as a JSON object. The following example specifies range as [(1, *, 2), (3, *, 4)] for 3 dimension objects, and bool expression as "A" AND ("B" OR "C").

```json
{
  "start_block": 1,
  "end_block": 10,
  "range": [[1, null, 2], [3, null, 4]],
  "bool": [["a"], ["b", "c"]]
}
```

The response is a JSON object like:

```json
{
  "result": ...,
  "vo": ...,
  "query_time_in_ms": ...,
  "vo_size": ... // in bytes
  "stats": ...,
  ...
}
```

Refer to the source code for their definitions.

### Verify

Pass the query response directly to the following endpoint for verification.

```
POST /verify
```

The response is a JSON object like:

```json
{
  "pass": true,
  "detail": ... // detail reason for failure
  "verify_time_in_ms": ...
}
```
