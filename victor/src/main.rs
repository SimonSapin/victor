fn main() -> Result<(), Box<std::error::Error>> {
    let mut args = std::env::args_os();
    let mut next = || args.next().ok_or("Usage: victor input.html output.pdf");
    let _self = next()?;
    let input = next()?;
    let output = next()?;
    let bytes = std::fs::read(&input)?;
    let doc = victor::dom::Document::parse_html(&bytes);
    let pdf = doc.to_pdf_bytes();
    std::fs::write(&output, &pdf)?;
    Ok(())
}