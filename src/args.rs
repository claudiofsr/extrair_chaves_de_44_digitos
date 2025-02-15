use clap::Parser;
use std::{fs, path::PathBuf, process};

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
    pub fn build() -> Result<Arguments, Box<dyn std::error::Error>> {
        let args: Arguments = Arguments::parse();
        args.validate_dir_path()?;
        Ok(args)
    }

    /// Validate directory paths
    fn validate_dir_path(&self) -> Result<(), Box<dyn std::error::Error>> {
        let paths = [&self.path];

        for dir_path in paths.into_iter().flatten() {
            if !std::path::Path::new(&dir_path).try_exists()? {
                eprintln!("fn validate_dir_path()");
                eprintln!("The path {dir_path:?} was not found!");
                process::exit(1);
            };

            if !dir_path.is_dir() {
                eprintln!("fn validate_dir_path()");
                eprintln!("{dir_path:?} is not a directory!");
                process::exit(1);
            }

            // Check if able to write inside directory
            let metadada = fs::metadata(dir_path)?;

            if metadada.permissions().readonly() {
                eprintln!("fn validate_dir_path()");
                eprintln!("No write permission");
                eprintln!("{dir_path:?} is readonly!");
                process::exit(1);
            }
        }

        Ok(())
    }
}
