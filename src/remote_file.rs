use std::{fs, io::Write};

pub fn is_valid_png(bytes: &[u8]) -> bool {
    let png_signature = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    bytes.len() >= png_signature.len() && &bytes[..png_signature.len()] == png_signature
}

pub async fn download_and_store_png(from: &str, to: &str) -> Result<(), anyhow::Error> {
    let response = reqwest::get(from).await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download file: {}",
            response.status()
        ));
    }

    let mut file = fs::File::create(to)?;
    let bytes = response.bytes().await?;
    if !is_valid_png(&bytes) {
        return Err(anyhow::anyhow!("File is not a valid PNG"));
    }

    file.write_all(&bytes)?;

    Ok(())
}
