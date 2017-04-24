extern crate data_encoding;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json as json;
extern crate tempdir;
extern crate tuf;
extern crate url;

use data_encoding::HEXLOWER;
use std::fs::{self, File, DirBuilder};
use std::io::Read;
use tempdir::TempDir;
use tuf::{Tuf, Config, Error};
use tuf::meta::{Key, KeyValue, KeyType};
use url::Url;

static VECTOR_META: &'static str = include_str!("./tuf-test-vectors/vectors/vector-meta.json");

#[derive(Deserialize)]
struct VectorMeta {
    repo: String,
    error: Option<String>,
    root_keys: Vec<RootKeyData>,
}

#[derive(Deserialize)]
struct RootKeyData {
    path: String,
    #[serde(rename = "type")]
    typ: String,
}

fn run_test_vector(test_path: &str) {
    let tempdir = TempDir::new("rust-tuf").expect("couldn't make temp dir");

    let vectors: Vec<VectorMeta> = json::from_str(VECTOR_META).expect("couldn't deserializd meta");

    let test_vector = vectors.iter()
        .filter(|v| v.repo == test_path)
        .collect::<Vec<&VectorMeta>>()
        .pop()
        .expect(format!("No repo named {}", test_path).as_str());

    let vector_path = format!("./tests/tuf-test-vectors/vectors/{}", test_vector.repo);

    for dir in vec!["metadata/latest", "metadata/archive", "targets"].iter() {
        DirBuilder::new()
            .recursive(true)
            .create(tempdir.path().join(dir))
            .expect(&format!("couldn't create path {}:", dir));
    }

    for file in vec!["root.json",
                     "targets.json",
                     "timestamp.json",
                     "snapshot.json"]
        .iter() {
        fs::copy(format!("{}/repo/{}", vector_path, file),
                 tempdir.path().join("metadata").join("latest").join(file))
            .expect(&format!("copy failed: {}", file));
    }

    fs::copy(format!("{}/repo/targets/file.txt", vector_path),
             tempdir.path().join("targets").join("file.txt"))
            .expect(&format!("copy failed for target"));

    let root_keys = test_vector.root_keys.iter()
        .map(|k| {
            let mut file = File::open(format!("{}/keys/{}", vector_path, k.path))
                .expect("couldn't open file");
            let mut key = String::new();
            file.read_to_string(&mut key).expect("couldn't read key");

            match k.typ.as_ref() {
                "ed25519" => {
                    let val = HEXLOWER.decode(key.replace("\n", "").as_ref())
                        .expect("key value not hex");
                    Key {
                        typ: KeyType::Ed25519,
                        value: KeyValue(val),
                    }
                },
                x => panic!("unknown key type: {}", x),
            }
        })
        .collect();

    let config = Config::build()
        .url(Url::parse("http://localhost:8080").expect("bad url"))
        .local_path(tempdir.into_path())
        .finish()
        .expect("bad config");

    match (Tuf::from_root_keys(root_keys, config), &test_vector.error) {
        (Ok(ref tuf), &None) => {
            assert_eq!(tuf.list_targets(), vec!["targets/file.txt".to_string()]);
            assert_eq!(tuf.verify_target("targets/file.txt"), Ok(()));
        },
        (Ok(ref tuf), &Some(ref err)) if err == &"TargetHashMismatch".to_string() => {
            assert_eq!(tuf.verify_target("targets/file.txt"), Err(Error::TargetHashMismatch));
        },
        (Ok(ref tuf), &Some(ref err)) if err == &"OversizedTarget".to_string() => {
            assert_eq!(tuf.verify_target("targets/file.txt"), Err(Error::OversizedTarget));
        },
        (Err(ref tuf_err), &Some(ref vector_err)) => {
            panic!("{:?} : {}", tuf_err, vector_err)
        },
        x => {
            panic!("{:?}", x)
        }
    }
}

mod vectors {
    use super::*;

    #[test]
    fn vector_001() {
        run_test_vector("001")
    }

    #[test]
    fn vector_002() {
        run_test_vector("002")
    }

    #[test]
    fn vector_005() {
        run_test_vector("005")
    }
}
