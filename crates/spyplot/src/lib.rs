// Desire: module that allows me to efficiently plot live data in the GPU
// The core idea: As we receive data, we rechunk it occasionally, so that we don't have
// too many draw operations.
// I'm thinking using 4096 points per chunk, and then we can draw 4096 points at a time.
// The final chunk will be allocated to that size, but will basically get frozen once it reaches that size.
pub mod lines;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
