use std::io::{self, Write};

struct Logger<'p, W: Write> {
    writer: W,
    prefix: &'p str,
}

impl<'p, W: Write> Logger<'p, W> {
    fn new(writer: W) -> Logger<'p, W> {
        Logger { writer, prefix: "" }
    }

    fn with_block<F>(&mut self, heading: &str, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Logger<&mut W>) -> io::Result<()>,
    {
        writeln!(self.writer, "┏ [{heading}]")?;
        let mut logger = Logger {
            writer: &mut self.writer,
            prefix: &format!("{}┃", self.prefix),
        };
        f(&mut logger)?;
        writeln!(self.writer, "┗ DONE")?;
        Ok(())
    }

    fn info(&mut self, message: &str) -> io::Result<()> {
        writeln!(self.writer, "{} {message}", self.prefix)
    }

    fn error(&mut self, message: &str) -> io::Result<()> {
        writeln!(self.writer, "{} {message}", self.prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_block_works() {
        let mut bytes: Vec<u8> = vec![];
        let mut logger = Logger::new(&mut bytes);
        logger
            .with_block("Setting up", |setup_log| {
                setup_log.info("Creating widget").unwrap();
                setup_log
                    .with_block("Reconfiguring sprite", |reconfigure_log| {
                        reconfigure_log.info("writing default")
                    })
                    .unwrap();
                setup_log.info("whatchamacalit unwrapped")
            })
            .unwrap();

        let actual = String::from_utf8(bytes).expect("Couldn't parse log output");
        let expected = "HEADINGSOMESTUFF".to_string();
        assert_eq!(expected, actual);
    }
}
