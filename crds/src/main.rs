use clap::{arg, Command, Parser, ValueEnum};
use crds::IngressRoute;
use kube::CustomResourceExt;

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Crd {
    #[clap(alias = "ingressroute", alias = "ingressRoute")]
    IngressRoute,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(value_enum)]
    crd: Crd,
}

fn cli() -> Command {
    Command::new("ferrix-crd")
        .about("Ferrix command line interface for managing CRD's")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("print")
                .about("Print a CRD")
                .arg(
                    arg!(<CRD>)
                        .value_parser(clap::value_parser!(Crd))
                        .required(true),
                )
                .arg(
                    arg!(--copy)
                        .help("Copy the output to clipboard")
                        .action(clap::ArgAction::SetTrue)
                        .required(false),
                ),
        )
}

fn main() {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("print", sub_matches)) => {
            let crd = sub_matches.get_one::<Crd>("CRD").unwrap();
            let s = match crd {
                Crd::IngressRoute => IngressRoute::crd(),
            };
            println!("{}", serde_yml::to_string(&s).unwrap());
            if sub_matches.get_flag("copy") {
                println!("CRD copied to clipboard");
            }
        }
        _ => unreachable!(),
    }
}
