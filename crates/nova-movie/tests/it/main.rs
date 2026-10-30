//! nova-movie's integration tests: the FFmpeg it links, decoding the
//! checked-in fixtures (`tests/fixtures/README.md`), the movie player
//! contract on them, and the stock movies when `NOVA_DATA` is set.

mod build;
mod common;
mod contract;
mod decode;
mod stock_movies;
