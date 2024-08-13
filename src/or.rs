#[derive(Debug)]
pub enum Or<L, R> {
    L(L),
    R(R),
}
