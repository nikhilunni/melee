//! Hit element discriminants, melee/lb/forward.h:40-69.
c_enum! {
    /// HitElement: selects damage behavior, hitlag modifiers and contact effects.
    pub enum HitElement: i32 {
        /// HitElement_Normal
        Normal = 0,
        /// HitElement_Fire
        Fire = 1,
        /// HitElement_Electric
        Electric = 2,
        /// HitElement_Slash
        Slash = 3,
        /// HitElement_Coin
        Coin = 4,
        /// HitElement_Ice
        Ice = 5,
        /// HitElement_Nap
        Nap = 6,
        /// HitElement_Sleep
        Sleep = 7,
        /// HitElement_Catch
        Catch = 8,
        /// HitElement_Ground
        Ground = 9,
        /// HitElement_Cape
        Cape = 10,
        /// HitElement_Inert
        Inert = 11,
        /// HitElement_Disable
        Disable = 12,
        /// HitElement_Dark
        Dark = 13,
        /// HitElement_Scball
        ScrewAttack = 14,
        /// HitElement_Lipstick
        Lipstick = 15,
        /// HitElement_Leadead
        ReDead = 16,
    }
}
