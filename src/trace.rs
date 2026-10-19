pub enum Op { Read, Write }

pub enum Event {
    Switch(u32),
    Exit(u32),
    Access { op: Op, addr: u64 },
}

pub fn parse_line(line: &str) -> Result<Option<Event>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("#") {
        return Ok(None);
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    // we already checked for empty line, so we know there are at least 2 parts
    if parts.len() != 2 {
        return Err(format!(
            "Invalid line: {}", line
        ))
    }
    let action = parts[0];
    let arg = parts[1];
    match action{
        "S" => {
            let pid = arg.parse::<u32>().map_err(|_| format!("Invalid PID: {}", arg))?;
            Ok(Some(Event::Switch(pid)))
        }
        "E" => {
            let pid = arg.parse::<u32>().map_err(|_| format!("Invalid PID: {}", arg))?;
            Ok(Some(Event::Exit(pid)))
        }
        "R" => {
            let hex_addr = arg.strip_prefix("0x").ok_or(format!("Invalid address: {}", arg))?;
            let addr = u64::from_str_radix(hex_addr, 16).map_err(|_| format!("Invalid address: {}", arg))?;
            Ok(Some(Event::Access { op: Op::Read, addr }))
        }
        "W" => {
            let hex_addr = arg.strip_prefix("0x").ok_or(format!("Invalid address: {}", arg))?;
            let addr = u64::from_str_radix(hex_addr, 16).map_err(|_| format!("Invalid address: {}", arg))?;
            Ok(Some(Event::Access { op: Op::Write, addr }))
        }
        // for every other thing...
        _ => {
            Err(format!("Invalid action: {}", action))
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_lines() {
        assert!(matches!(parse_line("S 1"), Ok(Some(Event::Switch(1)))));
        assert!(matches!(parse_line("E 2"), Ok(Some(Event::Exit(2)))));
        assert!(matches!(
            parse_line("R 0x1000"),
            Ok(Some(Event::Access { op: Op::Read, addr: 0x1000 }))
        ));
        assert!(matches!(
            parse_line("W 0x2040"),
            Ok(Some(Event::Access { op: Op::Write, addr: 0x2040 }))
        ));
    }

    #[test]
    fn skips_blank_and_comments() {
        assert!(matches!(parse_line(""), Ok(None)));
        assert!(matches!(parse_line("   "), Ok(None)));
        assert!(matches!(parse_line("# hi"), Ok(None)));
    }

    #[test]
    fn rejects_bad_lines() {
        for bad in ["X 5", "R banana", "S", "R 0x10 extra", "S -1"] {
            assert!(parse_line(bad).is_err(), "should reject {bad:?}");
        }
    }
}