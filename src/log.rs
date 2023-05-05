use std::{
    io::{self, Write},
    time::Instant,
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub(crate) struct Logger<'p, W: WriteColor> {
    writer: W,
    prefix: &'p str,
}

impl<'p, W: WriteColor> Logger<'p, W> {
    pub(crate) fn new(writer: W) -> Logger<'p, W> {
        Logger { writer, prefix: "" }
    }

    pub(crate) fn with_block<F, O, E>(&mut self, header: impl AsRef<str>, f: F) -> Result<O, E>
    where
        F: FnOnce(&mut Logger<&mut W>) -> Result<O, E>,
    {
        writeln!(self.writer, "{}┏[{}]", self.prefix, header.as_ref()).unwrap();
        let mut logger = Logger {
            writer: &mut self.writer,
            prefix: &format!("{}┃", self.prefix),
        };
        let start = Instant::now();
        let result = f(&mut logger);
        let duration = start.elapsed();
        match result {
            Ok(_) => writeln!(self.writer, "{}┗done in {:?}", self.prefix, duration).unwrap(),
            Err(_) => writeln!(self.writer, "{}┗errored in {:?}", self.prefix, duration).unwrap(),
        }
        result
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

    pub(crate) fn error(&mut self, hdr: impl AsRef<str>, msg: impl AsRef<str>) {
        write!(self.writer, "{}", self.prefix).unwrap();
        self.writer
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
            .unwrap();
        write!(self.writer, "[Error: {}]", hdr.as_ref()).unwrap();
        self.writer.reset().unwrap();
        writeln!(self.writer).unwrap();
        self.writer
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(false))
            .unwrap();
        write!(self.writer, "{}{}", self.prefix, msg.as_ref()).unwrap();
        self.writer.reset().unwrap();
        writeln!(self.writer).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use indoc::indoc;

    #[test]
    fn with_block_works() {
        let mut buffer = termcolor::Buffer::no_color();
        let mut logger = Logger::new(&mut buffer);
        logger.with_block("Setting up", |setup_log| {
            setup_log.info("Creating widget");
            setup_log.with_block("Reconfiguring sprite", |reconf_log| {
                Ok::<_, Infallible>(reconf_log.info("writing default"))
            });
            setup_log.info("whatchamacalit unwrapped");
            Ok::<_, Infallible>(())
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
