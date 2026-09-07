use std::env;
use std::fs;
use std::path::Path;

// Bakes data/*.tsv into static tables so lookups need no RPC and no runtime
// parsing. Both arrays are emitted sorted on their lookup key for binary search.
fn main() {
    println!("cargo:rerun-if-changed=data/registry-4663.tsv");
    println!("cargo:rerun-if-changed=data/feed-token-map.tsv");

    let mut assets = read_tsv("data/registry-4663.tsv", 6);
    for a in &mut assets {
        a[1] = addr(&a[1]);
    }
    assets.sort_by(|a, b| a[1].cmp(&b[1]));

    let mut feeds = read_tsv("data/feed-token-map.tsv", 4);
    for f in &mut feeds {
        f[1] = addr(&f[1]);
        f[2] = addr(&f[2]);
        f[3] = addr(&f[3]);
    }
    feeds.sort_by(|a, b| a[3].cmp(&b[3]));

    let mut src = String::new();
    src.push_str("pub static ASSETS: &[Asset] = &[\n");
    for a in &assets {
        let decimals: u32 = a[3].parse().expect("registry decimals");
        src.push_str(&format!(
            "    Asset {{ ticker: {:?}, token: {:?}, multiplier: {:?}, decimals: {}, status: {:?}, name: {:?} }},\n",
            a[0], a[1], a[2], decimals, a[4], a[5]
        ));
    }
    src.push_str("];\n\npub static FEEDS: &[Feed] = &[\n");
    for f in &feeds {
        src.push_str(&format!(
            "    Feed {{ ticker: {:?}, token: {:?}, proxy: {:?}, aggregator: {:?} }},\n",
            f[0], f[1], f[2], f[3]
        ));
    }
    src.push_str("];\n");

    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("registry_data.rs");
    fs::write(out, src).expect("write registry_data.rs");
}

fn read_tsv(path: &str, cols: usize) -> Vec<Vec<String>> {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let row: Vec<String> = l.split('\t').map(|s| s.trim().to_string()).collect();
            assert_eq!(row.len(), cols, "{path}: expected {cols} columns in {l:?}");
            row
        })
        .collect()
}

fn addr(s: &str) -> String {
    let a = s.to_ascii_lowercase();
    assert!(
        a.len() == 42 && a.starts_with("0x") && a[2..].bytes().all(|b| b.is_ascii_hexdigit()),
        "bad address {s:?}"
    );
    a
}
