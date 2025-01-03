# True Infinite Canvas

## What does true infinite mean?

Some other infinite canvases can struggle due to floating point precision problems. You can't zoom in past a certain point if you are far from the center of the canvas or else lines will be stuck to a grid. This project fixes that by storing everything in a quad tree meaning the only limit is device storage. It also avoids rendering outside of view thereby making it practial to store almost arbitrary amounts in it.

## Current project state

The project can currently best be described as pre-alpha (see `alpha-todo.txt`). It is therefore missing even basic features and has significant bugs.

## Project Goals
* Allow infinite zooming in and out on any part of the canvas without quality decrease
* Cross Platform
    * Windows
    * MacOS
    * Linux
    * Web
    * Ios
    * Android
* Performant
    * Every operation should be limited by, at maximum, visible detail
* Be a good resource for organizing arbitrary detail
    * High quality pen support
    * Layers
    * Basic Editing Tools
        * Draw
        * Erase
        * Select
        * Translate
        * Rotate
        * Scale
    * Display
        * Basic stroke
        * Basic shapes
        * Images

## Current Non-goals
* Support for multiple users
* Be a great tool for creating art


### Testing locally

Make sure you are using the latest version of stable rust by running `rustup update`.

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`

On Fedora Rawhide you need to run:

`dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`

### Web Locally

You can compile your app to [WASM](https://en.wikipedia.org/wiki/WebAssembly) and publish it as a web page.

We use [Trunk](https://trunkrs.dev/) to build for web target.
1. Install the required target with `rustup target add wasm32-unknown-unknown`.
2. Install Trunk with `cargo install --locked trunk`.
3. Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
4. Open `http://127.0.0.1:8080/index.html#dev` in a browser. See the warning below.

> `assets/sw.js` script will try to cache our app, and loads the cached version when it cannot connect to server allowing your app to work offline (like PWA).
> appending `#dev` to `index.html` will skip this caching, allowing us to load the latest builds during development.

### Web Deploy
1. Just run `trunk build --release`.
2. It will generate a `dist` directory as a "static html" website
3. Upload the `dist` directory to any of the numerous free hosting websites including [GitHub Pages](https://docs.github.com/en/free-pro-team@latest/github/working-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site).
4. we already provide a workflow that auto-deploys our app to GitHub pages if you enable it.
> To enable Github Pages, you need to go to Repository -> Settings -> Pages -> Source -> set to `gh-pages` branch and `/` (root).
>
> If `gh-pages` is not available in `Source`, just create and push a branch called `gh-pages` and it should be available.
>
> If you renamed the `main` branch to something else (say you re-initialized the repository with `master` as the initial branch), be sure to edit the github workflows `.github/workflows/pages.yml` file to reflect the change
