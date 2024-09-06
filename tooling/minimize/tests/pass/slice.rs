// Pass slice reference accross function boundaries
pub fn assert_some_elements(a: &[i32]) {
    assert!(a[1] == -40);
    assert!(a[2] == 30);
    assert!(a[3] == -20);
}

pub fn change_some_elements(a: &mut [u8]) {
    a[0] += 1;
    a[1] -= 1;
}

fn main() {
    let x: [i32; 5] = [50, -40, 30, -20, 10];
    let slice: &[i32] = &x;

    // Check indexablity
    assert_some_elements(slice);

    let mut a2 = [1, 2, 3, 4];
    change_some_elements(&mut a2);
    assert!(a2[0] == 2);

    assert!(slice.len() == 5);
    assert!(slice.iter().count() == 5);

    // Check the iterator in a for loop, checking alternating signs
    let mut sign = 1;
    for elem in slice {
        assert!(sign * elem > 0);
        sign *= -1;
    }
    assert!(sign == -1);

    // This is currently broken:
    // // Check subslicing, which uses `from_raw_parts`
    // let sub_slice = unsafe { core::slice::from_raw_parts::<'_, i32>(&slice[1] as *const i32, 4) };
    // // let sub_slice = &slice[1..];
    // assert!(sub_slice.len() == 4);
    // assert!(sub_slice[0] == -40);
    // let x = slice;
}
