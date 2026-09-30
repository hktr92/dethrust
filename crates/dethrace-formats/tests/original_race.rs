use dethrace_formats::{
    car_visual::CarVisualSpec,
    race::{GalleryRoster, OpponentCatalog, RaceCatalog, initial_player},
};
use std::path::PathBuf;

#[test]
#[ignore = "requires CARMAGEDDON_DIR with original Carmageddon assets"]
fn maim_street_roster_resolves_to_original_car_visuals() {
    let root = PathBuf::from(std::env::var_os("CARMAGEDDON_DIR").expect("set CARMAGEDDON_DIR"));
    let data = if root.join("DATA/RACES.TXT").exists() {
        root.join("DATA")
    } else {
        root
    };
    let races = RaceCatalog::parse(&std::fs::read(data.join("RACES.TXT")).unwrap()).unwrap();
    let opponents =
        OpponentCatalog::parse(&std::fs::read(data.join("OPPONENT.TXT")).unwrap()).unwrap();
    let (rank, player) = initial_player(&std::fs::read(data.join("GENERAL.TXT")).unwrap()).unwrap();
    assert_eq!(rank, 99);
    assert_eq!(player, "BLKEAGLE.TXT");
    let roster = GalleryRoster::resolve(&races, &opponents, "Maim Street", &player, 1).unwrap();
    assert_eq!(roster.track_file, "CITYA1.TXT");
    assert_eq!(roster.opponents.len(), 5);
    for car in std::iter::once(&roster.player_car_file)
        .chain(roster.opponents.iter().map(|op| &op.car_file))
    {
        let path = data.join("CARS").join(car);
        let spec = CarVisualSpec::parse(&std::fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(spec.definition.eq_ignore_ascii_case(car));
    }
}
