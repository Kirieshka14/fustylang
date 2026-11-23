// FustyLang Tsundere & Femboy Syntax Showcase (`nya`, `meow`, `purr`, `baka!`)

nya headpat_meter(pats: i64) -> i64 {
    if pats <= 0 {
        baka!("Hmph! Zero headpats?! Don't even talk to me! >///<");
    }
    return pats * 2;
}

let score = headpat_meter(7);
purr("Headpat power level:", score);

meow {
    purr("Inside meow unsafe block — is_meow =", is_meow());
}

// Uncomment to trigger a tsundere runtime panic:
// headpat_meter(0);

return score;
