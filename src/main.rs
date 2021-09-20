use clap::{App, Arg};
use futures::stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};
use tokio::io::AsyncWriteExt;
use url::Url;

fn access_token(
    consumer_key: String,
    consumer_secret: String,
    access_token: String,
    access_token_secret: String,
) -> egg_mode::Token {
    let api_token = egg_mode::KeyPair::new(consumer_key, consumer_secret);
    let access_token = egg_mode::KeyPair::new(access_token, access_token_secret);

    egg_mode::Token::Access {
        consumer: api_token,
        access: access_token,
    }
}

fn user_timeline(
    token: egg_mode::Token,
    user_id: egg_mode::user::UserID,
) -> egg_mode::tweet::Timeline {
    egg_mode::tweet::user_timeline(user_id, false, false, &token)
}

async fn get_urls(token: egg_mode::Token, username: String, max_image_count: u32) -> Vec<String> {
    let mut tweets_retrieved: u32 = 0;
    let mut urls: Vec<String> = vec![];

    let spinner = ProgressBar::new_spinner();
    spinner.set_draw_target(ProgressDrawTarget::stdout());
    spinner.enable_steady_tick(80);

    let user_id = egg_mode::user::UserID::ScreenName(username.to_owned().into());
    let mut timeline = user_timeline(token, user_id).with_page_size(200);

    'retrieval: loop {
        spinner.set_message(format!(
            "Retrieving tweets for user {} ({} tweets / {} images)...",
            username,
            tweets_retrieved,
            urls.len()
        ));
        match timeline.older(None).await {
            Ok((new_timeline, feed)) => {
                timeline = new_timeline;
                for tweet in &*feed {
                    if let Some(media) = &tweet.entities.media {
                        for entry in media {
                            if entry.media_type != egg_mode::entities::MediaType::Photo {
                                continue;
                            }

                            if entry.expanded_url.contains("/video/") {
                                // Skip every entry, which expanded_url has a /video/ segment.
                                // Unfortunately video thumbnails are presented with "media_type" photo :(
                                continue;
                            }

                            let url = entry.media_url.clone();
                            urls.push(url);
                            if max_image_count > 0 && urls.len() >= max_image_count as usize {
                                break 'retrieval;
                            }
                        }
                    }
                    tweets_retrieved += 1;
                }

                if let None = timeline.min_id {
                    // We are looping the tweet cycle
                    break;
                }
            }
            Err(_err) => {
                break;
            }
        }
    }

    spinner.finish_with_message(format!(
        "Tweets for user {} retrieved ({} tweets / {} images)...",
        username,
        tweets_retrieved,
        urls.len()
    ));

    urls
}

async fn download_urls(urls: Vec<String>, max_requests: u32, target_directory: String) {
    let multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::stdout());
    let main_progress = multi_progress.add(ProgressBar::new(urls.len() as u64));
    main_progress.set_prefix("Downloading Images");
    let mut spinners: Vec<ProgressBar> = vec![];
    for _ in 0..max_requests {
        let spinner = multi_progress.add(ProgressBar::new_spinner());
        spinner.enable_steady_tick(80);
        spinners.push(spinner);
    }

    // Ensure that the multiprogress is properly rendered.
    let multi_progress_join_handle =
        tokio::task::spawn_blocking(move || multi_progress.join().unwrap());

    let fetches = futures::stream::iter(urls.into_iter().enumerate().map(|(index, url)| {
        let spinner = &spinners[index % max_requests as usize];
        let progress = &main_progress;
        let target_directory = &target_directory;
        async move {
            spinner.set_message(format!("Downloading: {}", url));
            let response = reqwest::get(&url)
                .await
                .expect(&format!("Could not download url {}", url));
            let bytes = response.bytes().await.expect(&format!(
                "Could not retrieve download result for url {}",
                url
            ));
            let parsed_url =
                Url::parse(url.as_str()).expect(&format!("Could not parse URL: {}", url));
            match parsed_url.path().split("/").last() {
                Some(file_name) => {
                    let path = format!("{}/{}", target_directory, file_name);
                    let mut f = tokio::fs::File::create(&path)
                        .await
                        .expect(&format!("Could not open file for writing {}", path));
                    f.write_all(&bytes)
                        .await
                        .expect(&format!("Could not write file {}", path));
                }
                None => panic!("Could not extract filename from url {}", url),
            }
            progress.inc(1);
        }
    }))
    .buffer_unordered(max_requests as usize)
    .collect::<Vec<()>>();
    fetches.await;

    for spinner in spinners.iter() {
        spinner.finish_and_clear();
    }

    main_progress.finish();
    multi_progress_join_handle.await.unwrap();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let current_working_directory = std::env::current_dir().unwrap();
    let matches = App::new("Twitter Image Downloader")
        .version("1.0")
        .author("Jakob Westhoff <jakob@westhoffswelt.de>")
        .about("Download posted images from a given twitter user")
        .arg(
            Arg::with_name("consumer_key")
                .short("k")
                .long("consumer-key")
                .value_name("KEY")
                .help("Twiter API Consumer Key")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("consumer_secret")
                .short("c")
                .long("consumer-secret")
                .value_name("SECRET")
                .help("Twiter API Consumer Secret")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("access_token")
                .short("t")
                .long("access-token")
                .value_name("TOKEN")
                .help("Twiter API Access Token")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("access_token_secret")
                .short("s")
                .long("access-token-secret")
                .value_name("SECRET")
                .help("Twiter API Access Token Secret")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("number_of_images")
                .short("n")
                .long("max-images")
                .value_name("N")
                .help("Maximal number of images to download")
                .takes_value(true)
                .default_value("0"),
        )
        .arg(
            Arg::with_name("max_requests")
                .short("m")
                .long("max-requests")
                .value_name("N")
                .help("Maximal number of parallel download requests")
                .takes_value(true)
                .default_value("4"),
        )
        .arg(
            Arg::with_name("output_directory")
                .short("o")
                .long("output-directory")
                .value_name("DIRECTORY")
                .help("Directory to storage downloaded images in")
                .takes_value(true)
                .default_value(current_working_directory.to_str().unwrap()),
        )
        .arg(
            Arg::with_name("output_urls")
                .short("u")
                .long("output-url-list")
                .value_name("FILENAME")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("username")
                .help("Twitter username to download images from.")
                .value_name("USERNAME")
                .required(true)
                .index(1),
        )
        .get_matches();

    let output_directory = matches.value_of("output_directory").unwrap();
    std::fs::create_dir_all(output_directory.clone()).expect(
        format!(
            "Target directory '{:?}' could not be created.",
            output_directory
        )
        .as_str(),
    );
    let canonicalized_directory = std::fs::canonicalize(output_directory).unwrap();

    let output_urls = matches.value_of("output_urls");

    println!("Using output directory {:?}", canonicalized_directory);

    if let Some(filename) = output_urls {
        println!("Storing retrieved urls in {}", filename);
    }

    let username = matches.value_of("username").unwrap();
    let max_image_count = matches
        .value_of("number_of_images")
        .unwrap()
        .parse::<u32>()
        .unwrap();

    let token = access_token(
        matches.value_of("consumer_key").unwrap().to_string(),
        matches.value_of("consumer_secret").unwrap().to_string(),
        matches.value_of("access_token").unwrap().to_string(),
        matches.value_of("access_token_secret").unwrap().to_string(),
    );

    let urls = get_urls(token, username.to_string(), max_image_count).await;

    if let Some(filename) = output_urls {
        let mut f = tokio::fs::File::create(filename)
            .await
            .expect(&format!("Could not open file for writing {}", filename));
        for url in urls.iter() {
            f.write_all(format!("{}\n", url).as_bytes())
                .await
                .expect(&format!("Could not write to file {}", filename));
        }
        drop(f);
    }

    let max_requests = matches
        .value_of("max_requests")
        .unwrap()
        .parse::<u32>()
        .unwrap();

    download_urls(
        urls,
        max_requests,
        canonicalized_directory.to_str().unwrap().to_string(),
    )
    .await;

    println!("Everything done! Have fun.");
}
