use std::{
    fs::File,
    io,
};
use scrap::{Capturer, Display};


/// Takes screencaps of all available Displays.
pub fn take_screencaps() -> io::Result<()> {
    for (screen_number, display) in Display::all()?.into_iter().enumerate() {
        let mut capturer = Capturer::new(display)?;
        let (width, height) = (capturer.width(), capturer.height());

        let frame = capturer.frame()?;

        repng::encode(
            File::create(&format!("{}.png", screen_number))?,
            width as u32,
            height as u32,
            &*frame,
        )?;
    }

    Ok(())
}
