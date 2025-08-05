use clap::Parser;

#[derive(Debug, clap::Parser)]
struct Arguments {
    base: String,
    compare: String,
}

fn main() {
    let args = Arguments::parse();

    let dir = std::env::current_dir().unwrap();
    let repo = gix::open(dbg!(dir)).unwrap();

    let base_rev = repo.rev_parse(args.base.as_str()).unwrap();
    let compare_rev = repo.rev_parse(args.compare.as_str()).unwrap();

    if base_rev == compare_rev {
        eprintln!("cannot compare the same revision");
    }

    dbg!((base_rev, compare_rev));
}
