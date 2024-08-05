#[derive(Debug)]
pub enum Or<T1, T2> {
    T1(T1),
    T2(T2),
}
