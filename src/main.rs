use std::{env};
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use image::{ImageFormat};
use sha1::{Sha1, Digest};
use sqlx::{MySql, Pool};
use utils::get_rating;
use crate::utils::{get_rating_from_value};

mod default;
mod utils;

#[derive(Clone)]
struct State {
    conn: Pool<MySql>,
    resource_path: String,
}

#[async_std::main]
async fn main() -> tide::Result<()> {

    // 加载.env文件
    dotenv::dotenv().ok();

    // 读取环境变量
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let resource_path = env::var("RESOURCE_PATH").expect("RESOURCE_PATH must be set");
    let _secret_key = env::var("SECRET_KEY");

    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?; // 获取连接池

    let state = State {
        conn: pool,
        resource_path
    };

    let mut app = tide::with_state(state);

    app.at("/:md5").get(get_avatar);
    app.listen("127.0.0.1:9489").await?;

    Ok(())
}


async fn get_avatar(req: tide::Request<State>) -> tide::Result {

    let mut response = tide::Response::new(200);
    response.set_content_type("image/avif");
    let mut buffer: Option<Vec<u8>> = None;
    let now = chrono::Utc::now();

    let id = req.param("md5").unwrap_or("default");
    let r = req.query::<String>().ok();
    let rating = get_rating_from_value(r);
    let state = req.state();

    if !utils::is_hex_string(id) {
        return Ok(("What!\n".to_owned() + id).into());
    }

    let eid = id.to_lowercase();
    let obj = sqlx::query_as::<_,Avatar>("SELECT * FROM grsync_query WHERE eid = ? AND rating <= ? order by rating desc")
        .bind(&eid)
        .bind(rating)
        .fetch_one(&state.conn)
        .await;

    if let Ok(data) = obj {
        if data.last_update < now.timestamp() - 60 * 60 * 24 { let _ = sync_avatar(&data.eid, data.rating, data.sha1.clone(), &state.resource_path,&state.conn); }

        let mut path = None;
        if let Some(r) = data.resource { path = Some(r); }
        else if let Some(r) = data.sha1 { path = Some(r + ".avif"); }

        if let Some(p) = path
            { buffer = get_buffer(state.resource_path.to_owned() + &p); }
    } else {
        if let Ok(_) = sqlx::query("INSERT INTO grsync_avatar(eid,rating) VALUE(?, 3)")
            .bind(&eid)
            .execute(&state.conn)
            .await
        {
            if let Some(path) = sync_avatar(&eid, rating, None,&state.resource_path,&state.conn).await
                { buffer = get_buffer(state.resource_path.to_owned() + &path); }
        }

    }

    match buffer {
        Some(buf) => { response.set_body(buf) },
        None => response.set_body(default::DEFAULT_AVATAR)
    }
    Ok(response)
}


fn get_buffer(path: String) -> Option<Vec<u8>> {
    let file = File::open(path);
    if let Ok(mut file) = file {
        let mut buf = Vec::new();

        return match file.read_to_end(&mut buf) {
            Ok(_) => Some(buf),
            Err(_) => None
        }

    }

    None
}


async fn sync_avatar(eid: &str, rating: i8, origin_hash: Option<String>,dir: &str,conn: &Pool<MySql>) -> Option<String> {
    println!("now, sync EMD5-{}-{} citizen avatar", eid, get_rating(rating));
    let _ = sqlx::query("UPDATE grsync_avatar SET last_update = ? WHERE eid = ? AND rating = ?")
        .bind(chrono::Utc::now().timestamp())
        .bind(eid)
        .bind(rating)
        .execute(conn)
        .await;

    let client = surf::Client::new();
    let url = format!("https://gravatar.com/avatar/{}?r={}&s={}&d=404", eid, get_rating(rating), 512);
    let mut response = client
        .get(url.clone())
        .header("User-Agent", "grsync/0.1.0")
        .await.ok()?;
    let image_bytes= response.body_bytes().await.ok()?;

    let img = image::load_from_memory(&image_bytes).ok()?;

    let mut avif_bytes = Vec::new();
    // 使用 Cursor
    let mut cursor = Cursor::new(&mut avif_bytes);
    img.write_to(&mut cursor, ImageFormat::Avif).ok()?;

    // 计算 SHA1
    let mut hasher = Sha1::new();
    hasher.update(&avif_bytes);
    let sha1_result = hasher.finalize();
    let sha1_hex = hex::encode(sha1_result);

    if origin_hash == Some(sha1_hex.clone()) {return None}

    let obj = sqlx::query_as::<_,Image>("SELECT sha1, resource FROM grsync_resource WHERE sha1 = ?")
        .bind(&sha1_hex)
        .fetch_one(conn)
        .await;

    if let Ok(data) = obj {

        let _ = sqlx::query("UPDATE grsync_avatar SET resource = ? WHERE eid = ? AND rating = ?")
            .bind(data.sha1.clone())
            .bind(eid)
            .bind(rating)
            .execute(conn)
            .await;

        return match data.resource {
            None => Some(data.sha1 + ".avif"),
            Some(p) => Some(p)
        };
    }


    let path = format!("{}/{}.avif", dir,sha1_hex);
    let output_path = Path::new(&path);
    let _ = save_img(eid, rating, &sha1_hex, &output_path, avif_bytes, &url,conn).await;

    Some(sha1_hex + ".avif")
}



#[derive(Debug, sqlx::FromRow)]
struct Avatar {
    eid: String,
    rating: i8,
    sha1: Option<String>,
    resource: Option<String>,
    size: Option<i32>,

    last_update: i64
}


#[derive(Debug, sqlx::FromRow)]
struct Image
{
    sha1: String,
    resource: Option<String>,
}

async fn save_img(eid: &str, rating: i8,sha1:&str,output_path:&Path, bytes: Vec<u8>, origin_url: &str,conn: &Pool<MySql>) -> Option<()>
{

    let mut file = File::create(output_path).ok()?;
    file.write_all(&bytes).ok()?;

    let _ = sqlx::query("INSERT INTO grsync_resource VALUES(?,?,?,?)")
        .bind(sha1).bind(sha1.to_owned() + ".avif").bind(512).bind(origin_url)
        .execute(conn)
        .await;

    let _ = sqlx::query("UPDATE grsync_avatar SET resource = ? WHERE eid = ? AND rating = ?")
        .bind(sha1)
        .bind(eid)
        .bind(rating)
        .execute(conn)
        .await;
    Some(())
}
