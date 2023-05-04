use std::io::{self, Write};
use termcolor::{Color, ColorSpec, WriteColor};

pub(crate) struct Logger<'p, W: WriteColor> {
    writer: W,
    prefix: &'p str,
}

impl<'p, W: WriteColor> Logger<'p, W> {
    pub(crate) fn new(writer: W) -> Logger<'p, W> {
        Logger { writer, prefix: "" }
    }

    pub(crate) fn with_block<F, T>(&mut self, header: impl AsRef<str>, f: F) -> T
    where
        F: FnOnce(&mut Logger<&mut W>) -> T,
    {
        writeln!(self.writer, "{}┏[{}]", self.prefix, header.as_ref()).unwrap();
        let mut logger = Logger {
            writer: &mut self.writer,
            prefix: &format!("{}┃", self.prefix),
        };
        let ret = f(&mut logger);
        writeln!(self.writer, "{}┗done", self.prefix).unwrap();
        ret
    }

    pub(crate) fn header(&mut self, hdr: impl AsRef<str>) {
        write!(self.writer, "{}", self.prefix).unwrap();
        self.writer
            .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))
            .unwrap();
        write!(self.writer, "[{}]", hdr.as_ref()).unwrap();
        self.writer.reset().unwrap();
        writeln!(self.writer).unwrap();
    }

    pub(crate) fn info(&mut self, msg: impl AsRef<str>) {
        writeln!(self.writer, "{}{}", self.prefix, msg.as_ref()).unwrap();
    }

    pub(crate) fn error(&mut self, message: &str) -> io::Result<()> {
        writeln!(self.writer, "{}{message}", self.prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn with_block_works() {
        let mut buffer = termcolor::Buffer::no_color();
        let mut logger = Logger::new(&mut buffer);
        logger.with_block("Setting up", |setup_log| {
            setup_log.info("Creating widget");
            setup_log.with_block("Reconfiguring sprite", |reconf_log| {
                reconf_log.info("writing default");
            });
            setup_log.info("whatchamacalit unwrapped");
        });

        let actual = String::from_utf8(buffer.into_inner()).expect("Couldn't parse log output");
        assert_eq!(
            actual,
            indoc! {"
                ┏[Setting up]
                ┃Creating widget
                ┃┏[Reconfiguring sprite]
                ┃┃writing default
                ┃┗done
                ┃whatchamacalit unwrapped
                ┗done
            "}
        );
    }
}
