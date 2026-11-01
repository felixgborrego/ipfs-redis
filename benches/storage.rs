//! Benchmarks for the in-memory storage engine and the command execution
//! layer (parse -> execute -> serialize).

use std::io::{BufReader, BufWriter, Cursor};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use divan::{black_box, Bencher};
use ipfsredis::cmds;
use ipfsredis::protocol::{Cmd, Data};
use ipfsredis::storage::{Config, Db};

fn main() {
    divan::main();
}

const SIZES: &[usize] = &[10, 100, 1000];

fn populated_db(n: usize, ttl: bool) -> Db {
    let db = Db::new(Config::default());
    let expiration = SystemTime::now() + Duration::from_secs(3600);
    for i in 0..n {
        db.set(
            &format!("key:{i}"),
            &format!("value:{i}"),
            ttl.then_some(expiration),
        );
    }
    db
}

mod db {
    use super::{black_box, populated_db, Bencher, Config, Db, Duration, SystemTime, SIZES};

    #[divan::bench(args = SIZES)]
    fn set(bencher: Bencher, n: usize) {
        let keys: Vec<String> = (0..n).map(|i| format!("key:{i}")).collect();
        bencher
            .with_inputs(|| Db::new(Config::default()))
            .bench_local_values(|db| {
                for key in &keys {
                    db.set(black_box(key), black_box("some-value"), None);
                }
                db
            });
    }

    #[divan::bench(args = SIZES)]
    fn set_with_ttl(bencher: Bencher, n: usize) {
        let keys: Vec<String> = (0..n).map(|i| format!("key:{i}")).collect();
        let expiration = SystemTime::now() + Duration::from_secs(3600);
        bencher
            .with_inputs(|| Db::new(Config::default()))
            .bench_local_values(|db| {
                for key in &keys {
                    db.set(black_box(key), black_box("some-value"), Some(expiration));
                }
                db
            });
    }

    #[divan::bench(args = SIZES)]
    fn get_hit(bencher: Bencher, n: usize) {
        let db = populated_db(n, false);
        let keys: Vec<String> = (0..n).map(|i| format!("key:{i}")).collect();
        bencher.bench_local(|| {
            for key in &keys {
                black_box(db.get(black_box(key)));
            }
        });
    }

    #[divan::bench(args = SIZES)]
    fn get_hit_with_ttl(bencher: Bencher, n: usize) {
        let db = populated_db(n, true);
        let keys: Vec<String> = (0..n).map(|i| format!("key:{i}")).collect();
        bencher.bench_local(|| {
            for key in &keys {
                black_box(db.get(black_box(key)));
            }
        });
    }

    #[divan::bench(args = SIZES)]
    fn get_miss(bencher: Bencher, n: usize) {
        let db = populated_db(n, false);
        let keys: Vec<String> = (0..n).map(|i| format!("missing:{i}")).collect();
        bencher.bench_local(|| {
            for key in &keys {
                black_box(db.get(black_box(key)));
            }
        });
    }

    #[divan::bench(args = SIZES)]
    fn keys(bencher: Bencher, n: usize) {
        let db = populated_db(n, false);
        bencher.bench_local(|| black_box(db.keys(black_box("*"))));
    }
}

mod execute {
    use super::{black_box, cmds, populated_db, Arc, Bencher, Cmd, Data};

    fn bulk(s: &str) -> Data {
        Data::BulkString(s.to_string())
    }

    #[divan::bench]
    fn ping(bencher: Bencher) {
        let state = Arc::new(populated_db(0, false));
        bencher.bench_local(|| cmds::execute(black_box(Cmd::Ping), &state).unwrap());
    }

    #[divan::bench]
    fn echo(bencher: Bencher) {
        let state = Arc::new(populated_db(0, false));
        bencher
            .with_inputs(|| Cmd::Echo {
                args: vec![bulk("hello world")],
            })
            .bench_local_values(|cmd| cmds::execute(cmd, &state).unwrap());
    }

    #[divan::bench]
    fn set(bencher: Bencher) {
        let state = Arc::new(populated_db(1000, false));
        bencher
            .with_inputs(|| Cmd::Set {
                args: vec![bulk("key:500"), bulk("new-value")],
            })
            .bench_local_values(|cmd| cmds::execute(cmd, &state).unwrap());
    }

    #[divan::bench]
    fn set_with_px(bencher: Bencher) {
        let state = Arc::new(populated_db(1000, false));
        bencher
            .with_inputs(|| Cmd::Set {
                args: vec![
                    bulk("key:500"),
                    bulk("new-value"),
                    bulk("PX"),
                    bulk("60000"),
                ],
            })
            .bench_local_values(|cmd| cmds::execute(cmd, &state).unwrap());
    }

    #[divan::bench]
    fn get(bencher: Bencher) {
        let state = Arc::new(populated_db(1000, false));
        bencher
            .with_inputs(|| Cmd::Get {
                args: vec![bulk("key:500")],
            })
            .bench_local_values(|cmd| cmds::execute(cmd, &state).unwrap());
    }

    #[divan::bench]
    fn info_replication(bencher: Bencher) {
        let state = Arc::new(populated_db(0, false));
        bencher
            .with_inputs(|| Cmd::Info {
                args: vec![bulk("replication")],
            })
            .bench_local_values(|cmd| cmds::execute(cmd, &state).unwrap());
    }
}

/// End-to-end request handling: parse a RESP request, execute it against the
/// database and serialize the response, as done for every client request.
#[divan::bench(args = ["GET", "SET"])]
fn request_roundtrip(bencher: Bencher, op: &str) {
    let request: &[u8] = match op {
        "GET" => b"*2\r\n$3\r\nGET\r\n$7\r\nkey:500\r\n",
        _ => b"*3\r\n$3\r\nSET\r\n$7\r\nkey:500\r\n$9\r\nnew-value\r\n",
    };
    let state = Arc::new(populated_db(1000, false));
    bencher.bench_local(|| {
        let mut reader = BufReader::new(Cursor::new(black_box(request)));
        let cmd = Data::parse_cmd(&mut reader).unwrap();
        let response = cmds::execute(cmd, &state).unwrap();
        let mut writer = BufWriter::new(Vec::with_capacity(64));
        response.write_resp(&mut writer).unwrap();
        black_box(writer.into_inner().unwrap())
    });
}
