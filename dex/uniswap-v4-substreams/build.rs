use anyhow::Result;
use regex::Regex;
use std::fs;
use substreams_ethereum::Abigen;

fn sanitize_abi(contents: &str) -> String {
    let regex = Regex::new(r#"("\w+"\s?:\s?")_(\w+")"#).unwrap();
    let sanitized = regex.replace_all(contents, "${1}u_${2}");
    let re = Regex::new(r"_+").unwrap();
    re.replace_all(&sanitized, |caps: &regex::Captures| {
        let count = caps[0].len();
        if count <= 1 {
            return "_".to_string();
        }
        format!("{}_", "_u".repeat(count - 1))
    })
    .to_string()
}

fn main() -> Result<(), anyhow::Error> {
    let contracts = [
        ("abi/PoolManager.json", "src/abi/pool_manager.rs", "PoolManager"),
        (
            "abi/PositionManager.json",
            "src/abi/position_manager.rs",
            "PositionManager",
        ),
    ];

    fs::create_dir_all("src/abi")?;

    for (abi_path, out_path, name) in contracts {
        let contents =
            fs::read_to_string(abi_path).unwrap_or_else(|e| panic!("read {abi_path}: {e}"));
        let sanitized = sanitize_abi(&contents);
        Abigen::from_bytes(name, sanitized.as_bytes())?
            .generate()?
            .write_to_file(out_path)?;
    }

    Ok(())
}
