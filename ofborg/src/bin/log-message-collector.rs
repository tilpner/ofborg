extern crate ofborg;
extern crate amqp;
extern crate env_logger;

use std::env;
use std::path::PathBuf;

use amqp::Session;
use amqp::Table;

use ofborg::config;
use ofborg::worker;
use ofborg::tasks;
use amqp::Basic;

fn main() {
    let cfg = config::load(env::args().nth(1).unwrap().as_ref());
    ofborg::setup_log();


    let mut session = Session::open_url(&cfg.rabbitmq.as_uri()).unwrap();
    println!("Connected to rabbitmq");
    {
        println!("About to open channel #1");
        let hbchan = session.open_channel(1).unwrap();

        println!("Opened channel #1");

        tasks::heartbeat::start_on_channel(hbchan, cfg.whoami());
    }

    let mut channel = session.open_channel(2).unwrap();

    let queue_name = channel
        .queue_declare(
            "",
            false, // passive
            false, // durable
            true, // exclusive
            true, // auto_delete
            false, //nowait
            Table::new(),
        )
        .expect("Failed to declare an anon queue for log collection!")
        .queue;

    channel
        .queue_bind(
            queue_name.as_ref(),
            "logs",
            "*.*".as_ref(),
            false,
            Table::new(),
        )
        .unwrap();


    channel
        .basic_consume(
            worker::new(tasks::log_message_collector::LogMessageCollector::new(
                PathBuf::from(cfg.log_storage.clone().unwrap().path),
                100,
            )),
            queue_name,
            format!("{}-log-collector", cfg.whoami()),
            false,
            false,
            false,
            false,
            Table::new(),
        )
        .unwrap();

    channel.start_consuming();

    println!("Finished consuming?");

    channel.close(200, "Bye").unwrap();
    println!("Closed the channel");
    session.close(200, "Good Bye");
    println!("Closed the session... EOF");

}
