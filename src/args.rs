use clap::Parser;
use std::path::PathBuf;

// https://stackoverflow.com/questions/74068168/clap-rs-not-printing-colors-during-help
fn get_styles() -> clap::builder::Styles {

    let cyan   = anstyle::Color::Ansi(anstyle::AnsiColor::Cyan);
    let green  = anstyle::Color::Ansi(anstyle::AnsiColor::Green);
    let yellow = anstyle::Color::Ansi(anstyle::AnsiColor::Yellow);

    clap::builder::Styles::styled()
        .placeholder(
            anstyle::Style::new()
                .fg_color(Some(yellow))
        )
        .usage(
            anstyle::Style::new()
                .fg_color(Some(cyan))
                .bold()
        )
        .header(
            anstyle::Style::new()
                .fg_color(Some(cyan))
                .bold()
                .underline()
        )
        .literal(
            anstyle::Style::new()
                .fg_color(Some(green))
        )
}

// https://docs.rs/clap/latest/clap/struct.Command.html#method.help_template
const APPLET_TEMPLATE: &str = "\
{before-help}
{about-with-newline}
{usage-heading} {usage}

{all-args}
{after-help}";

#[derive(Parser, Debug)]
#[command(
    // Read from `Cargo.toml`
    author, version, about,
    long_about = None,
    next_line_help = true,
    help_template = APPLET_TEMPLATE,
    styles=get_styles(),
)]
pub struct Arguments {
    /// Set maximum depth to recursively search EFD files
    ///
    /// Avoid descending into directories when the depth is exceeded
    #[arg(short('m'), long("max_depth"), required = false)]
    pub max_depth: Option<usize>,

    /// Set the xml file path, otherwise recursively search
    /// for xml files in the current directory
    #[arg(short('p'), long("path"), required = false)]
    pub path: Option<PathBuf>,

    /// Show total execution time
    #[arg(short('t'), long("time"), default_value_t = false)]
    pub time: bool,

    /// Show intermediate runtime messages.
    #[arg(short('v'), long("verbose"), default_value_t = false)]
    pub verbose: bool,
}

impl Arguments {
    /// Build Arguments struct
    pub fn build() -> Result<Arguments, Box<dyn std::error::Error>> {
        let args: Arguments = Arguments::parse();
        Ok(args)
    }
}
