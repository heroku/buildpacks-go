use std::io;
use std::io::Write;

impl<'l> Write for BaseLogger<'l> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

struct BaseLogger<'b>(&'b mut dyn io::Write);
struct BlockLogger<'b> {
    logger: &'b mut BaseLogger<'b>,
    prefix: &'static str,
}

impl<'b> BaseLogger<'b> {
    fn log_block<F>(&'b mut self, heading: &str, f: F)
    where
        F: FnOnce(&mut BlockLogger),
    {
        self.info(&format!("- [{heading}]"));
        f(&mut BlockLogger {
            logger: self,
            prefix: "|",
        });
        self.info(&format!("- [{heading}]"));
        // self.info(&format!("- [{heading}]"));
    }
    fn info(&mut self, msg: &str) {
        writeln!(self.0, "{msg}").unwrap();
    }
}

impl<'l> BlockLogger<'l> {
    fn info(&mut self, text: &str) {
        self.logger.info(&format!("{}{text}", self.prefix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_block_works() {
        let mut bytes: Vec<u8> = vec![];
        BaseLogger(&mut bytes).log_block("HEADING", |log| {
            log.info("SOME");
            log.info("STUFF");
        });

        let actual = String::from_utf8(bytes).expect("Couldn't parse log output");
        let expected = "HEADINGSOMESTUFF".to_string();
        assert_eq!(expected, actual);
    }
}
