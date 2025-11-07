use crate::error::{MyError, MyResult};
use clap::Parser;
use std::{fs, path::PathBuf};

// https://stackoverflow.com/questions/74068168/clap-rs-not-printing-colors-during-help
fn get_styles() -> clap::builder::Styles {
    let cyan = anstyle::Color::Ansi(anstyle::AnsiColor::Cyan);
    let green = anstyle::Color::Ansi(anstyle::AnsiColor::Green);
    let yellow = anstyle::Color::Ansi(anstyle::AnsiColor::Yellow);

    clap::builder::Styles::styled()
        .placeholder(anstyle::Style::new().fg_color(Some(yellow)))
        .usage(anstyle::Style::new().fg_color(Some(cyan)).bold())
        .header(
            anstyle::Style::new()
                .fg_color(Some(cyan))
                .bold()
                .underline(),
        )
        .literal(anstyle::Style::new().fg_color(Some(green)))
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
    /// Set the minimum depth to search for identical files.
    ///
    /// depth >= min_depth
    #[arg(short('d'), long("min_depth"), required = false, default_value_t = 0)]
    pub min_depth: usize,

    /// Set the maximum depth to search for identical files.
    ///
    /// Avoid descending into directories when the depth is exceeded.
    ///
    /// depth <= max_depth
    #[arg(
        short('D'), long("max_depth"), 
        required = false,
        default_value_t = usize::MAX,
        hide_default_value = true,
    )]
    pub max_depth: usize,

    /// Set the SPED EFD txt file path, otherwise recursively search
    /// for txt files in the current directory
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
    pub fn build() -> MyResult<Arguments> {
        let args: Arguments = Arguments::parse();
        args.validate_dir_path()?;
        Ok(args)
    }

    /// Validate directory paths
    fn validate_dir_path(&self) -> MyResult<()> {
        let paths = [&self.path];

        for dir_path in paths.into_iter().flatten() {
            if !dir_path.try_exists()? {
                return Err(MyError::PathNotFound(dir_path.clone()));
            };

            if !dir_path.is_dir() {
                return Err(MyError::NotADirectory(dir_path.clone()));
            }

            // Check if able to write inside directory
            let metadada = fs::metadata(dir_path)?;

            if metadada.permissions().readonly() {
                return Err(MyError::ReadOnlyDirectory(dir_path.clone()));
            }
        }

        Ok(())
    }
}
