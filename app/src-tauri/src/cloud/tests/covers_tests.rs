//! Covers through the cloud (D82, part 4): after the operations, each device swaps the cover files of its cloud
//! libraries with the server. The very same file reaches the other devices; nothing goes to a library linked
//! elsewhere, and a server that lists more than the library's covers gets nothing more fetched.

use super::*;

fn covers_in(name: &str) -> Covers {
    let dir = std::env::temp_dir().join(format!(
        "kosmotheka-cloud-covers-{name}-{}-{}",
        std::process::id(),
        ops::new_id()
    ));
    Covers::new(dir).unwrap()
}

/// A small cover, different for each seed.
fn cover(covers: &Covers, seed: u8) -> String {
    let picture = image::RgbImage::from_fn(60, 90, |x, y| {
        image::Rgb([
            seed.wrapping_mul(47),
            u8::try_from((x * 4) % 256).unwrap(),
            u8::try_from((y * 2) % 256).unwrap() ^ seed.wrapping_mul(91),
        ])
    });
    let mut png = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut png, image::ImageFormat::Png).unwrap();
    covers.store_cover(png.get_ref()).unwrap()
}

fn add_with_cover(device: &Database, library: &str, title: &str, cover: &str) {
    catalog::create_book(
        &mut device.lock().unwrap(),
        library,
        &edition(title),
        Some(cover),
    )
    .unwrap();
}

#[tokio::test]
async fn a_cover_goes_up_from_one_device_and_down_to_another_the_same() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib, _hugo, c) = laptop_and_hugos_phone_in_sync(&cloud).await;
    let (on_laptop, on_phone) = (covers_in("laptop"), covers_in("phone"));
    let tersoal = cloud.api("tersoal").on(&laptop);
    let hugo = cloud.api("hugo").on(&phone);
    let name = cover(&on_laptop, 1);
    add_with_cover(&laptop, &lib, "Catan", &name);
    sync_library(&laptop, &tersoal, &lib).await.unwrap();

    assert_eq!(
        exchange_covers(&laptop, &tersoal, &on_laptop)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        cloud.cover_in(&c, &name),
        on_laptop.read(&name),
        "uploaded as it is"
    );

    sync_all(&phone, &hugo).await.unwrap();
    assert_eq!(exchange_covers(&phone, &hugo, &on_phone).await.unwrap(), 1);
    assert_eq!(
        on_phone.read(&name),
        on_laptop.read(&name),
        "the very same file"
    );
    assert_eq!(
        exchange_covers(&phone, &hugo, &on_phone).await.unwrap(),
        0,
        "nothing left"
    );
}

#[tokio::test]
async fn a_cover_whose_bytes_are_not_the_named_one_is_not_kept() {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib, _hugo, c) = laptop_and_hugos_phone_in_sync(&cloud).await;
    let (on_laptop, on_phone) = (covers_in("laptop-lie"), covers_in("phone-lie"));
    let tersoal = cloud.api("tersoal").on(&laptop);
    let hugo = cloud.api("hugo").on(&phone);
    let name = cover(&on_laptop, 2);
    add_with_cover(&laptop, &lib, "Catan", &name);
    sync_library(&laptop, &tersoal, &lib).await.unwrap();
    // A server that hands another image under the cover's name.
    let other = on_laptop.read(&cover(&on_laptop, 3)).unwrap();
    cloud.put_cover(&c, &name, other);

    sync_all(&phone, &hugo).await.unwrap();

    assert_eq!(exchange_covers(&phone, &hugo, &on_phone).await.unwrap(), 0);
    assert!(!on_phone.has(&name));
}

#[tokio::test]
async fn a_library_that_is_not_in_this_cloud_swaps_no_cover_with_it() {
    let cloud = FakeCloud::default();
    let api = cloud.api("ana");
    let (ana, library) = device_with_family();
    let covers = covers_in("unlinked");
    add_with_cover(&ana, &library, "Catan", &cover(&covers, 4));

    assert_eq!(exchange_covers(&ana, &api, &covers).await.unwrap(), 0);
    assert_eq!(
        cloud.cover_calls(),
        0,
        "not linked: the server hears nothing"
    );

    let other_server = FakeCloud::default();
    let elsewhere = other_server.api("ana");
    upload_settled(&ana, &elsewhere, &library).await;
    let mut on_this = cloud.api("ana");
    on_this.server = "https://another.example".to_owned();
    assert_eq!(exchange_covers(&ana, &on_this, &covers).await.unwrap(), 0);
    assert_eq!(
        cloud.cover_calls(),
        0,
        "linked to another server: this one hears nothing"
    );
}
