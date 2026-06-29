use super::*;

pub(crate) fn write_mcp(stdout: &mut impl Write, value: &Value) -> Result<()> {
    writeln!(stdout, "{}", serde_json::to_string(value)?)?;
    stdout.flush()?;
    Ok(())
}
