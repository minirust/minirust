fn test_int_cast() {
    #[repr(i16)]
    enum ReprEnum {
        V1 = -12,
        V2 = -42,
    }
    
    fn convert_repr_enum(e: ReprEnum) -> i16 {
        e as i16
    }

    assert!(convert_repr_enum(ReprEnum::V1) == -12);
    assert!(convert_repr_enum(ReprEnum::V2) == -42);
}

fn test_overaligned_int_cast() {
    #[repr(align(8))]
    enum Aligned {
        Zero = 0,
        One = 1,
    }

    assert!(Aligned::Zero as u8 == 0);
    assert!(Aligned::One as u8 == 1);
}

#[allow(unused)]
fn test_big_enum() {
    macro_rules! fooN {
        ($cur:ident $prev:ty) => {
            #[allow(dead_code)]
            enum $cur {
                Empty,
                First($prev),
                Second($prev),
                Third($prev),
                Fourth($prev),
            }
        }
    }
    
    fooN!(Foo0 ());
    fooN!(Foo1 Foo0);
    fooN!(Foo2 Foo1);
    fooN!(Foo3 Foo2);
    fooN!(Foo4 Foo3);
    fooN!(Foo5 Foo4);
    fooN!(Foo6 Foo5);
    fooN!(Foo7 Foo6);
    fooN!(Foo8 Foo7);
    fooN!(Foo9 Foo8);
    fooN!(Foo10 Foo9);
    fooN!(Foo11 Foo10);
    fooN!(Foo12 Foo11);
    fooN!(Foo13 Foo12);
    fooN!(Foo14 Foo13);
    fooN!(Foo15 Foo14);
    fooN!(Foo16 Foo15);
    fooN!(Foo17 Foo16);
    fooN!(Foo18 Foo17);
    fooN!(Foo19 Foo18);
    fooN!(Foo20 Foo19);
    fooN!(Foo21 Foo20);
    fooN!(Foo22 Foo21);
    fooN!(Foo23 Foo22);
    fooN!(Foo24 Foo23);
    fooN!(Foo25 Foo24);
    fooN!(Foo26 Foo25);
    fooN!(Foo27 Foo26);
    
    let _foo = Foo27::Empty;
}

fn test_full_enum() {
    #[repr(i8)]
    pub enum X4 {
        _0 = -128, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15, _16,
        _17, _18, _19, _20, _21, _22, _23, _24, _25, _26, _27, _28, _29, _30, _31, _32,
        _33, _34, _35, _36, _37, _38, _39, _40, _41, _42, _43, _44, _45, _46, _47, _48,
        _49, _50, _51, _52, _53, _54, _55, _56, _57, _58, _59, _60, _61, _62, _63, _64,
        _65, _66, _67, _68, _69, _70, _71, _72, _73, _74, _75, _76, _77, _78, _79, _80,
        _81, _82, _83, _84, _85, _86, _87, _88, _89, _90, _91, _92, _93, _94, _95, _96,
        _97, _98, _99, _100, _101, _102, _103, _104, _105, _106, _107, _108, _109, _110, _111, _112,
        _113, _114, _115, _116, _117, _118, _119, _120, _121, _122, _123, _124, _125, _126, _127, _128,
        _129, _130, _131, _132, _133, _134, _135, _136, _137, _138, _139, _140, _141, _142, _143, _144,
        _145, _146, _147, _148, _149, _150, _151, _152, _153, _154, _155, _156, _157, _158, _159, _160,
        _161, _162, _163, _164, _165, _166, _167, _168, _169, _170, _171, _172, _173, _174, _175, _176,
        _177, _178, _179, _180, _181, _182, _183, _184, _185, _186, _187, _188, _189, _190, _191, _192,
        _193, _194, _195, _196, _197, _198, _199, _200, _201, _202, _203, _204, _205, _206, _207, _208,
        _209, _210, _211, _212, _213, _214, _215, _216, _217, _218, _219, _220, _221, _222, _223, _224,
        _225, _226, _227, _228, _229, _230, _231, _232, _233, _234, _235, _236, _237, _238, _239, _240,
        _241, _242, _243, _244, _245, _246, _247, _248, _249, _250, _251, _252, _253, _254, _255,
    }

    assert!(X4::_15 as i8 == -113)
}

fn main() {
    test_int_cast();
    test_overaligned_int_cast();
    // FIXME: cache type translation
    // test_big_enum();
    test_full_enum();
}
