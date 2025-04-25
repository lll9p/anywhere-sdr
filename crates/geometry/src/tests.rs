use crate::{coordinates::*, traits::LocationMath};
const LLH: [f64; 3] = [35.274_143_229, 137.014_853_084, 99.998];
const XYZ: [f64; 3] = [-3_813_477.954, 3_554_276.552, 3_662_785.237];
const EPS: f64 = 1e-8;
#[test]
fn test_geometry_location2efef() {
    let xyz = [
        f64::from_bits(13_923_324_196_484_912_872),
        f64::from_bits(4_706_606_011_222_523_641),
        f64::from_bits(13_929_576_035_448_519_812),
    ];
    // xyz = [-1676694.4690794293, 4515052.0724484855, -4167476.3179927487]
    let location = Location::from(&LLH);
    let ecef = Ecef::from(&location);
    let ecef_from_xyz = Ecef::from(&xyz);
    println!("Ecef fro old: {ecef_from_xyz:?}");
    println!("Ecef from new: {ecef:?}");
    assert!(
        ecef.precise(&ecef_from_xyz, EPS),
        "Not equal! {:#?}",
        ecef - &ecef_from_xyz
    );
}
#[test]
fn test_geometry_ecef2location() {
    let llh = [
        f64::from_bits(4_603_720_481_224_739_772),
        f64::from_bits(4_612_567_283_934_169_376),
        f64::from_bits(4_636_737_350_692_634_624),
    ];
    // xyz = [0.6156477194111782, 2.391360502574699, 100.00084324367344]
    let ecef = Ecef::from(&XYZ);
    let location = Location::from(&ecef);
    let location_from_llh = Location::from(&llh);
    println!("Location from old: {location_from_llh:?}");
    println!("Location from new: {location:?}");
    assert!(
        location.precise(&location_from_llh, EPS),
        "Not equal! {:#?}",
        location - location_from_llh
    );
}
#[test]
fn test_geometry_ltcmat() {
    let tmat = [
        [
            f64::from_bits(4_597_406_541_513_150_448),
            f64::from_bits(13_827_093_489_331_282_323),
            f64::from_bits(13_828_338_932_311_870_902),
        ],
        [
            f64::from_bits(4_606_618_994_045_393_047),
            f64::from_bits(4_599_942_923_006_032_601),
            f64::from_bits(0_000_000_000_000_000_000),
        ],
        [
            f64::from_bits(13_821_772_391_745_090_068),
            f64::from_bits(4_604_542_057_699_443_572),
            f64::from_bits(13_827_463_570_854_459_512),
        ],
    ];

    let location = Location::from(&LLH);
    let tmat_from_location = location.ltcmat();
    for (vec0, vec1) in tmat.iter().zip(&tmat_from_location) {
        for (i0, i1) in vec0.iter().zip(vec1) {
            assert!((i0 - i1).abs() <= EPS, "Not equal!");
        }
    }
}
#[test]
fn test_geometry_ecef2neu() {
    let tmat = Location::from(&LLH).ltcmat();
    let neu = [
        f64::from_bits(13_931_381_818_169_503_716),
        f64::from_bits(13_925_646_393_143_350_753),
        f64::from_bits(4_697_507_633_163_841_812),
    ];
    let ecef = Ecef::from(&XYZ);
    let neu = Neu::from(&neu);
    let neu_from_ecef = Neu::from_ecef(&ecef, tmat);
    println!("Neu from old: {neu:?}");
    println!("Neu from new: {neu_from_ecef:?}");
    assert!(neu.precise(&neu_from_ecef, EPS), "Not equal!",);
}
#[test]
fn test_geometry_neu2azel() {
    let neu = [
        f64::from_bits(13_931_381_818_169_503_716),
        f64::from_bits(13_925_646_393_143_350_753),
        f64::from_bits(4_697_507_633_163_841_812),
    ];
    let azel = [
        f64::from_bits(4_615_116_355_893_774_375),
        f64::from_bits(4_595_463_099_674_307_653),
    ];
    let neu = Neu::from(&neu);
    let azel = Azel::from(&azel);
    let azel_new = Azel::from(&neu);
    println!("Azel from old: {azel:?}");
    println!("Azel from new: {azel_new:?}");
    assert!(
        (azel.az - azel_new.az).abs() <= EPS
            && (azel.el - azel_new.el).abs() <= EPS,
        "Not equal!"
    );
}
