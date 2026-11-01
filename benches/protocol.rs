//! Benchmarks for the RESP protocol layer: parsing incoming commands and
//! serializing responses.

use std::io::{BufReader, BufWriter, Cursor};

use divan::{black_box, Bencher};
use ipfsredis::protocol::{Cmd, Data};

fn main() {
    divan::main();
}

const SIZES: &[usize] = &[1, 10, 100, 1000];

/// Encode a list of strings as a RESP array of bulk strings.
fn resp_array(items: &[String]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", items.len());
    for item in items {
        out.push_str(&format!("${}\r\n{item}\r\n", item.len()));
    }
    out.into_bytes()
}

fn bulk_items(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("value-{i:08}")).collect()
}

mod parse_cmd {
    use super::{black_box, resp_array, BufReader, Cursor, Data};

    fn bench_cmd(bencher: divan::Bencher, items: &[&str]) {
        let items: Vec<String> = items.iter().map(|s| (*s).to_string()).collect();
        let input = resp_array(&items);
        bencher.bench(|| {
            let mut reader = BufReader::new(Cursor::new(black_box(input.as_slice())));
            black_box(Data::parse_cmd(&mut reader).unwrap())
        });
    }

    #[divan::bench]
    fn ping(bencher: divan::Bencher) {
        bench_cmd(bencher, &["PING"]);
    }

    #[divan::bench]
    fn get(bencher: divan::Bencher) {
        bench_cmd(bencher, &["GET", "user:1000:session"]);
    }

    #[divan::bench]
    fn set(bencher: divan::Bencher) {
        bench_cmd(bencher, &["SET", "user:1000:session", "some-session-token"]);
    }

    #[divan::bench]
    fn set_with_px(bencher: divan::Bencher) {
        bench_cmd(
            bencher,
            &[
                "SET",
                "user:1000:session",
                "some-session-token",
                "PX",
                "60000",
            ],
        );
    }

    #[divan::bench]
    fn pipeline_100_cmds(bencher: divan::Bencher) {
        let mut input = Vec::new();
        for i in 0..100 {
            let items = vec!["SET".to_string(), format!("key:{i}"), format!("value:{i}")];
            input.extend(resp_array(&items));
        }
        bencher.bench(|| {
            let mut reader = BufReader::new(Cursor::new(black_box(input.as_slice())));
            for _ in 0..100 {
                black_box(Data::parse_cmd(&mut reader).unwrap());
            }
        });
    }
}

#[divan::bench(args = SIZES)]
fn parse_bulk_array(bencher: Bencher, n: usize) {
    let input = resp_array(&bulk_items(n));
    bencher.bench(|| {
        let mut reader = BufReader::new(Cursor::new(black_box(input.as_slice())));
        black_box(Data::parse(&mut reader).unwrap())
    });
}

mod write_resp {
    use super::{black_box, bulk_items, BufWriter, Data, SIZES};

    fn bench_write(bencher: divan::Bencher, data: &Data) {
        bencher.bench(|| {
            let mut writer = BufWriter::new(Vec::with_capacity(1024));
            black_box(data).write_resp(&mut writer).unwrap();
            black_box(writer.into_inner().unwrap())
        });
    }

    #[divan::bench]
    fn simple_string(bencher: divan::Bencher) {
        bench_write(bencher, &Data::ok_response());
    }

    #[divan::bench]
    fn bulk_string(bencher: divan::Bencher) {
        bench_write(bencher, &Data::BulkString("x".repeat(256)));
    }

    #[divan::bench]
    fn null_bulk_string(bencher: divan::Bencher) {
        bench_write(bencher, &Data::NullBuilkString);
    }

    #[divan::bench(args = SIZES)]
    fn array(bencher: divan::Bencher, n: usize) {
        let data = Data::Array(bulk_items(n).into_iter().map(Data::BulkString).collect());
        bench_write(bencher, &data);
    }

    #[divan::bench]
    fn full_resync_4kb(bencher: divan::Bencher) {
        let data = Data::FullResyncBinaryConent(
            Box::new(Data::SimpleString(
                "FULLRESYNC 8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb 0".to_string(),
            )),
            vec![0xAB; 4096],
        );
        bench_write(bencher, &data);
    }
}

#[divan::bench]
fn cmd_set_to_data(bencher: Bencher) {
    let cmd = Cmd::Set {
        args: vec![
            Data::BulkString("user:1000:session".to_string()),
            Data::BulkString("some-session-token".to_string()),
            Data::BulkString("PX".to_string()),
            Data::BulkString("60000".to_string()),
        ],
    };
    bencher.bench(|| black_box(&cmd).to_data().unwrap());
}
