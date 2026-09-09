//! Semantic fighter parts from `ft/forward.h:255-311`.

c_enum! {
    /// `Fighter_Part`; absent parts are represented by the mapping, not the enum.
    pub enum FtPart: i32 {
        /// `FtPart_TopN`
        TopN = 0,
        /// `FtPart_TransN`
        TransN = 1,
        /// `FtPart_XRotN`
        XRotN = 2,
        /// `FtPart_YRotN`
        YRotN = 3,
        /// `FtPart_HipN`
        HipN = 4,
        /// `FtPart_WaistN`
        WaistN = 5,
        /// `FtPart_LLegJA`
        LLegJA = 6,
        /// `FtPart_LLegJ`
        LLegJ = 7,
        /// `FtPart_LKneeJ`
        LKneeJ = 8,
        /// `FtPart_LFootJA`
        LFootJA = 9,
        /// `FtPart_LFootJ`
        LFootJ = 10,
        /// `FtPart_RLegJA`
        RLegJA = 11,
        /// `FtPart_RLegJ`
        RLegJ = 12,
        /// `FtPart_RKneeJ`
        RKneeJ = 13,
        /// `FtPart_RFootJA`
        RFootJA = 14,
        /// `FtPart_RFootJ`
        RFootJ = 15,
        /// `FtPart_BustN`
        BustN = 16,
        /// `FtPart_LShoulderN`
        LShoulderN = 17,
        /// `FtPart_LShoulderJA`
        LShoulderJA = 18,
        /// `FtPart_LShoulderJ`
        LShoulderJ = 19,
        /// `FtPart_LArmJ`
        LArmJ = 20,
        /// `FtPart_LHandN`
        LHandN = 21,
        /// `FtPart_L1stNa`
        L1stNa = 22,
        /// `FtPart_L1stNb`
        L1stNb = 23,
        /// `FtPart_L2ndNa`
        L2ndNa = 24,
        /// `FtPart_L2ndNb`
        L2ndNb = 25,
        /// `FtPart_L3rdNa`
        L3rdNa = 26,
        /// `FtPart_L3rdNb`
        L3rdNb = 27,
        /// `FtPart_L4thNa`
        L4thNa = 28,
        /// `FtPart_L4thNb`
        L4thNb = 29,
        /// `FtPart_LThumbNa`
        LThumbNa = 30,
        /// `FtPart_LThumbNb`
        LThumbNb = 31,
        /// `FtPart_LHandNb`
        LHandNb = 32,
        /// `FtPart_NeckN`
        NeckN = 33,
        /// `FtPart_HeadN`
        HeadN = 34,
        /// `FtPart_RShoulderN`
        RShoulderN = 35,
        /// `FtPart_RShoulderJA`
        RShoulderJA = 36,
        /// `FtPart_RShoulderJ`
        RShoulderJ = 37,
        /// `FtPart_RArmJ`
        RArmJ = 38,
        /// `FtPart_RHandN`
        RHandN = 39,
        /// `FtPart_R1stNa`
        R1stNa = 40,
        /// `FtPart_R1stNb`
        R1stNb = 41,
        /// `FtPart_R2ndNa`
        R2ndNa = 42,
        /// `FtPart_R2ndNb`
        R2ndNb = 43,
        /// `FtPart_R3rdNa`
        R3rdNa = 44,
        /// `FtPart_R3rdNb`
        R3rdNb = 45,
        /// `FtPart_R4thNa`
        R4thNa = 46,
        /// `FtPart_R4thNb`
        R4thNb = 47,
        /// `FtPart_RThumbNa`
        RThumbNa = 48,
        /// `FtPart_RThumbNb`
        RThumbNb = 49,
        /// `FtPart_RHandNb`
        RHandNb = 50,
        /// `FtPart_ThrowN`
        ThrowN = 51,
        /// `FtPart_TransN2`
        TransN2 = 52,
        /// `FtPart_56`
        Unknown56 = 56,
        /// `FtPart_109`
        Unknown109 = 109,
    }
}
