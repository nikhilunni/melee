# Particle gate: scenarios that differ from retail (2026-10-12)

`melee-sim gate --particles <scenario>` compares the port's particle system with
the scenario's particle dump on every tick (`docs/ORACLE.md`). This is the sweep
of main at de6702a0 with the entry-warp fix: 2584 scenarios with a retail trace.
The check is opt-in until the 103 scenarios below are fixed; each passes the
fighter and item gate. Regenerate with a sweep of `gate --particles`.

```
2584 scenarios, 5538 s summed
  1769  exact
   394  exact (no display clock in the dump: lists compared as bags)
   148  exact (a second run under the dump's display cadence)
   115  fails as in baseline
   103  PARTICLE FAULT (baseline gate passes)
    35  not compared: dump not compared: its pass is not the tick trace's game: its seed differs from record N
    14  no dump
     4  not compared: dump not compared: its pass wrote N records of M
     1  particle divergence (baseline gate already fails)
     1  exact (the dump was capped)

   35  NOT COMPARED dump not compared: its pass is not the tick trace's game: its seed differs from record N
         samus_missile_super_air_fd_fox4: dump not compared: its pass is not the tick trace's game: its seed differs from record 200 (0x58d229fa, the tick trace has 0xca5678a7)
         samus_missile_super_fd_fox4: dump not compared: its pass is not the tick trace's game: its seed differs from record 188 (0x57db7fec, the tick trace has 0x823316a4)
         slp_dl_donkeykong_ganondorf_gs_r1_t4900: dump not compared: its pass is not the tick trace's game: its seed differs from record 2233 (0xa5ef0d49, the tick trace has 0xa71edab1)
         slp_dl_donkeykong_ganondorf_r1_t4900: dump not compared: its pass is not the tick trace's game: its seed differs from record 2233 (0xa5ef0d49, the tick trace has 0xa71edab1)
         slp_dl_fox_falco_r1_t1600: dump not compared: its pass is not the tick trace's game: its seed differs from record 839 (0x4a12dcc6, the tick trace has 0x9ce5ad4e)
         slp_dl_marth_link_r115833_lag3265_t3400: dump not compared: its pass is not the tick trace's game: its seed differs from record 357 (0xa8a36924, the tick trace has 0x1340cf4d)
         slp_dl_marth_link_r115833_t3400: dump not compared: its pass is not the tick trace's game: its seed differs from record 357 (0xa8a36924, the tick trace has 0x1340cf4d)
         slp_dl_peach_younglink_r1_t8700: dump not compared: its pass is not the tick trace's game: its seed differs from record 1407 (0xf76c8068, the tick trace has 0x7dc380c9)
         slp_dl_sheik_captainfalcon_lag3014_t3300: dump not compared: its pass is not the tick trace's game: its seed differs from record 3075 (0xdd1e9b9b, the tick trace has 0x10da09f3)
         slp_dl_sheik_captainfalcon_r1_t3300: dump not compared: its pass is not the tick trace's game: its seed differs from record 3074 (0x85a6c6f0, the tick trace has 0xb8c44bb8)
         slp_fd_falco_yoshi_lag9869_t10000: dump not compared: its pass is not the tick trace's game: its seed differs from record 6038 (0x89182465, the tick trace has 0xe5699f4d)
         slp_fd_falco_yoshi_r1_t10000: dump not compared: its pass is not the tick trace's game: its seed differs from record 6038 (0x89182465, the tick trace has 0xe5699f4d)
         slp_fd_marth_sheik_r1_t6500: dump not compared: its pass is not the tick trace's game: its seed differs from record 2455 (0xd02967f2, the tick trace has 0x7465fffa)
         slp_fd_samus_pikachu_r1_t4250: dump not compared: its pass is not the tick trace's game: its seed differs from record 312 (0xf5159daa, the tick trace has 0xd1ef07a2)
         slp_fod_level_falco_zelda_r1_t9100: dump not compared: its pass is not the tick trace's game: its seed differs from record 1231 (0x5d6e2afa, the tick trace has 0x52157002)
         slp_fod_level_fox_jigglypuff_r1_t13500: dump not compared: its pass is not the tick trace's game: its seed differs from record 3278 (0x7eef612e, the tick trace has 0xd1c41ab6)
         slp_fod_samus_marth_lag7810_t7900: dump not compared: its pass is not the tick trace's game: its seed differs from record 191 (0xd2ba0351, the tick trace has 0x846ba19e)
         slp_fod_samus_marth_r1_t7900: dump not compared: its pass is not the tick trace's game: its seed differs from record 191 (0xd2ba0351, the tick trace has 0x846ba19e)
         slp_fod_yoshi_yoshi_r1_t10100: dump not compared: its pass is not the tick trace's game: its seed differs from record 2730 (0x2574e727, the tick trace has 0xb99e4d4f)
         slp_ps_falco_marth_frz_lag2983_t3200: dump not compared: its pass is not the tick trace's game: its seed differs from record 1441 (0x3d879604, the tick trace has 0x5e0bfb4c)
         slp_ps_falco_marth_frz_r1_t3200: dump not compared: its pass is not the tick trace's game: its seed differs from record 1441 (0x3d879604, the tick trace has 0x5e0bfb4c)
         slp_ps_fox_drmario_wide9263_nopause_r1_t7200: dump not compared: its pass is not the tick trace's game: its seed differs from record 2078 (0x27aaf467, the tick trace has 0x7c71f316)
         slp_ps_fox_drmario_wide9263_r1_t9800: dump not compared: its pass is not the tick trace's game: its seed differs from record 2078 (0x27aaf467, the tick trace has 0x7c71f316)
         slp_ps_gameandwatch_falco_r112449_t8200: dump not compared: its pass is not the tick trace's game: its seed differs from record 2047 (0x7a3a93bd, the tick trace has 0xf0083da5)
         slp_ps_gameandwatch_falco_r1_t7100: dump not compared: its pass is not the tick trace's game: its seed differs from record 2047 (0x7a3a93bd, the tick trace has 0xf0083da5)
         slp_ps_iceclimbers_samus_r191618_t6900: dump not compared: its pass is not the tick trace's game: its seed differs from record 239 (0xb5f1af70, the tick trace has 0x541a049d)
         slp_ps_link_luigi_r175018_t10500: dump not compared: its pass is not the tick trace's game: its seed differs from record 250 (0xecd21e67, the tick trace has 0x1b7443a2)
         slp_ps_link_luigi_t10500: dump not compared: its pass is not the tick trace's game: its seed differs from record 5991 (0xd998e15e, the tick trace has 0xa710c877)
         slp_ps_mario_pikachu_r1_t6400: dump not compared: its pass is not the tick trace's game: its seed differs from record 2172 (0x334ed73d, the tick trace has 0xf28e3125)
         slp_ps_marth_peach_wide9263_r1_t4600: dump not compared: its pass is not the tick trace's game: its seed differs from record 4457 (0xec20b99a, the tick trace has 0xd372c2a2)
         slp_ps_marth_peach_wide_r1_t4600: dump not compared: its pass is not the tick trace's game: its seed differs from record 4458 (0x4f284646, the tick trace has 0xa09a86ce)
         slp_ps_samus_captainfalcon_wide_r1_t2850: dump not compared: its pass is not the tick trace's game: its seed differs from record 210 (0x16f83ead, the tick trace has 0x9dd3e2b2)
         slp_ys_fox_marth_lag5874_t6000: dump not compared: its pass is not the tick trace's game: its seed differs from record 4139 (0x23b0e226, the tick trace has 0x31296eae)
         slp_ys_fox_marth_r1_t6000: dump not compared: its pass is not the tick trace's game: its seed differs from record 4139 (0x23b0e226, the tick trace has 0x31296eae)
         slp_ys_jigglypuff_falco_t4400: dump not compared: its pass is not the tick trace's game: its seed differs from record 2810 (0xf645bf28, the tick trace has 0x6d9edef0)
   20  particles.generator[].position[] | retail bank 0 program 411 | port bank 0 program 411
         corpus_v3_fd_ganondorf_fox4_e0fcf0c70_p0: tick 168: -21.543062210083008 (0xC1AC5831) vs -19.463529586791992 (0xC19BB54F)
         corpus_v3_fd_ganondorf_fox4_e1502cb40_p0: tick 550: -39.88213348388672 (0xC21F874E) vs -39.095149993896484 (0xC21C616F)
         corpus_v3_fd_ganondorf_fox4_e3ef41ca4_p2: tick 1035: -48.50902557373047 (0xC242093E) vs -48.07378387451172 (0xC2404B8E)
         corpus_v3_fd_ganondorf_fox4_ee62c6106_p0: tick 2628: -35.707340240478516 (0xC20ED451) vs -37.33981704711914 (0xC2155BF9)
         corpus_v3_fd_ganondorf_fox4_ef89b3e70_p0: tick 291: 27.43914794921875 (0x41DB8360) vs 25.359615325927734 (0x41CAE07E)
         corpus_v3_fd_mewtwo_fox4_e0fcf0c70_p0: tick 217: -13.44010066986084 (0xC1570AA7) vs -11.444676399230957 (0xC1371D65)
         corpus_v3_fd_mewtwo_fox4_e1b05b110_p2: tick 1384: 50.95806121826172 (0x424BD50E) vs 49.35490417480469 (0x42456B6C)
         corpus_v3_fd_mewtwo_fox4_e266e1150_p1: tick 3446: -74.62889862060547 (0xC29541FF) vs -73.34111022949219 (0xC292AEA6)
         corpus_v3_fd_mewtwo_fox4_e50814092_p0: tick 460: -23.720592498779297 (0xC1BDC3C6) vs -22.69906997680664 (0xC1B597B2)
         corpus_v3_fd_mewtwo_fox4_e7ff378da_p0: tick 572: -71.20201110839844 (0xC28E676E) vs -71.9216079711914 (0xC28FD7DD)
         corpus_v3_fd_mewtwo_fox4_e9a8e5048_p0: tick 255: 70.03173065185547 (0x428C103F) vs 69.3121337890625 (0x428A9FD0)
         corpus_v3_fd_mewtwo_fox4_eb8786a38_p0: tick 285: -20.74202537536621 (0xC1A5EFAB) vs -20.30678367614746 (0xC1A2744B)
         corpus_v3_fd_mewtwo_fox4_ec13743d5_p0: tick 627: 2.8367512226104736 (0x40358D55) vs 1.0947015285491943 (0x3F8C1F2E)
         corpus_v3_fd_mewtwo_fox4_ef89b3e70_p0: tick 1704: -26.926902770996097 (0xC1D76A4C) vs -28.757932662963867 (0xC1E6103F)
         ganon_sideb_airhit: tick 306: 59.377532958984375 (0x426D8298) vs 61.372955322265625 (0x42757DE8)
         ganon_sideb_hit: tick 293: 57.73271942138672 (0x4266EE4E) vs 59.81224822998047 (0x426F3FBE)
         mewtwo_ball_cancel_fd_fox4: tick 339: 59.37145233154297 (0x426D7C5E) vs 59.42121887207031 (0x426DAF54)
         mewtwo_ball_full_fd_fox4: tick 420: 59.741512298583984 (0x426EF74F) vs 58.95452880859375 (0x426BD170)
         mewtwo_ball_reflect_hit_fd_fox4: tick 266: 16.889554977416992 (0x41871DCF) vs 17.945981979370117 (0x418F915F)
         mewtwo_ball_tap_fd_fox4: tick 226: 59.37773513793945 (0x426D82CD) vs 59.67755126953125 (0x426EB5D0)
   13  particles.generator[].position[] | retail bank 0 program 409 | port bank 0 program 409
         corpus_v3_fd_ganondorf_fox4_e0d368f02_p1: tick 1036: 3.884336233139038 (0x407898F7) vs 5.963869094848633 (0x40BED804)
         corpus_v3_fd_ganondorf_fox4_e46703f61_p0: tick 1306: 45.01102828979492 (0x42340B4B) vs 43.989501953125 (0x422FF540)
         corpus_v3_fd_ganondorf_fox4_eae52f0ea_p0: tick 2138: 58.962493896484375 (0x426BD998) vs 57.00843048095703 (0x426408A2)
         corpus_v3_fd_ganondorf_fox4_ebe7703ed_p0: tick 118: -5.895148754119873 (0xC0BCA50F) vs -6.682136535644531 (0xC0D5D410)
         corpus_v3_fd_ganondorf_fox4_edafcfddf_p0: tick 2236: 57.81562042236328 (0x42674332) vs 59.769683837890625 (0x426F1428)
         corpus_v3_fd_ganondorf_fox4_ee98155f3_p0: tick 267: 15.05597686767578 (0x4170E548) vs 12.976444244384766 (0x414F9F84)
         corpus_v3_fd_ganondorf_fox4_eecfcf1ea_p1: tick 1741: -64.8038558959961 (0xC2819B93) vs -68.36094665527344 (0xC288B8CE)
         corpus_v3_fd_mewtwo_fox4_e4213e6a5_p0: tick 720: -18.24506950378418 (0xC191F5E7) vs -19.1154727935791 (0xC198EC7D)
         corpus_v3_fd_mewtwo_fox4_e70b3084c_p0: tick 378: 7.861885070800781 (0x40FB9490) vs 5.866461753845215 (0x40BBBA0E)
         corpus_v3_fd_mewtwo_fox4_e720659b1_p1: tick 955: -77.48969268798828 (0xC29AFAB9) vs -79.56922149658203 (0xC29F2371)
         corpus_v3_fd_mewtwo_fox4_ece1e43b7_p0: tick 2253: 53.063812255859375 (0x42544158) vs 51.068389892578125 (0x424C4608)
         ganon_downb_hit: tick 279: 59.741512298583984 (0x426EF74F) vs 58.95452880859375 (0x426BD170)
         ganon_punch_hit: tick 341: 59.741512298583984 (0x426EF74F) vs 58.95452880859375 (0x426BD170)
   12  particles.generator[].position[] | retail bank 0 program 427 | port bank 0 program 427
         corpus_v3_fd_sheik_fox4_ebe7703ed_p0: tick 599: -61.85846328735352 (0xC2776F11) vs -57.57734298706055 (0xC2664F33)
         corpus_v3_fd_zelda_fox4_e005a4f43_p1: tick 370: 5.157094478607178 (0x40A506EB) vs 9.438215255737305 (0x411702EE)
         corpus_v3_fd_zelda_fox4_e0dee256e_p1: tick 1029: -42.08234786987305 (0xC2285453) vs -43.19961166381836 (0xC22CCC67)
         corpus_v3_fd_zelda_fox4_e50814092_p2: tick 399: -40.30656433105469 (0xC22139EC) vs -34.67853546142578 (0xC20AB6D2)
         corpus_v3_fd_zelda_fox4_e89a89d0e_p2: tick 940: -78.44786071777344 (0xC29CE54E) vs -72.81983184814453 (0xC291A3C1)
         corpus_v3_fd_zelda_fox4_e9943b4ab_p1: tick 2343: 20.181455612182617 (0x41A1739F) vs 19.064189910888672 (0x41988376)
         corpus_v3_fd_zelda_fox4_e9943b4ab_p2: tick 489: 22.70357131958008 (0x41B5A0EA) vs 21.586305618286133 (0x41ACB0C1)
         corpus_v3_fd_zelda_fox4_edafcfddf_p2: tick 775: -51.046234130859375 (0xC24C2F58) vs -52.16349792480469 (0xC250A76C)
         corpus_v3_fd_zelda_fox4_ee62c6106_p0: tick 692: 15.090337753295898 (0x41717206) vs 15.153409957885742 (0x4172745E)
         sheikzelda_nayru_fd_zelda: tick 129: -63.37264633178711 (0xC27D7D97) vs -57.7446174621582 (0xC266FA7D)
         sheikzelda_zd_nayru_air_fd_zelda: tick 135: -62.10902404785156 (0xC2786FA4) vs -57.827903747558594 (0xC2674FC6)
         sheikzelda_zd_nayru_reflect_fd_zelda: tick 127: -63.37264633178711 (0xC27D7D97) vs -57.7446174621582 (0xC266FA7D)
   11  particles.generator[].position[] | retail bank 0 program 410 | port bank 0 program 410
         corpus_v3_fd_ganondorf_fox4_e255c070a_p1: tick 3459: -34.34015655517578 (0xC2095C52) vs -33.87163543701172 (0xC2077C8E)
         corpus_v3_fd_ganondorf_fox4_e573e2d95_p0: tick 709: 53.435211181640625 (0x4255BDA8) vs 55.514739990234375 (0x425E0F18)
         corpus_v3_fd_ganondorf_fox4_eb4935276_p0: tick 379: -52.98931121826172 (0xC253F50E) vs -53.42455291748047 (0xC255B2BE)
         corpus_v3_fd_ganondorf_fox4_ee133b82f_p0: tick 449: -46.55865478515625 (0xC23A3C10) vs -48.63818359375 (0xC2428D80)
         corpus_v3_fd_mewtwo_fox4_e2af099ca_p0: tick 389: -39.98112869262695 (0xC21FECAD) vs -42.0606575012207 (0xC2283E1D)
         corpus_v3_fd_mewtwo_fox4_e2b9e1400_p1: tick 680: -57.448875427246094 (0xC265CBA6) vs -58.73666000366211 (0xC26AF257)
         corpus_v3_fd_mewtwo_fox4_ec0a10b25_p2: tick 5812: 23.33180809020996 (0x41BAA78B) vs 21.69932746887207 (0x41AD9839)
         corpus_v3_fd_mewtwo_fox4_ee98155f3_p2: tick 2190: 51.21636199951172 (0x424CDD8E) vs 53.21178436279297 (0x4254D8DE)
         corpus_v3_fd_mewtwo_fox4_ef9b6d16d_p0: tick 2101: -45.60089111328125 (0xC2366750) vs -45.551124572753906 (0xC236345A)
         mewtwo_ball_air_fd_fox4: tick 292: 59.37761688232422 (0x426D82AE) vs 60.3590202331543 (0x42716FA3)
         mewtwo_throw_f_fd_fox4: tick 274: 73.71614837646484 (0x42936EAB) vs 72.11299133300781 (0x429039DA)
    7  particles.generator[].position[] | retail bank 14 program 14004 | port bank 14 program 14004
         corpus_v3_fd_iceclimbers_fox4_e1a144038_p1: tick 1112: 0 (0x00000000) vs -49.10874938964844 (0xC2446F5C)
         corpus_v3_fd_iceclimbers_fox4_e23bd2614_p2: tick 1312: 0 (0x00000000) vs -39.3099365234375 (0xC21D3D60)
         corpus_v3_fd_iceclimbers_fox4_e4ff31316_p1: tick 1055: 0 (0x00000000) vs 23.46392822265625 (0x41BBB620)
         corpus_v3_fd_iceclimbers_peach4_e0696dc6c_p1: tick 795: 0 (0x00000000) vs 36.383811950683594 (0x42118906)
         corpus_v3_ps_iceclimbers_peach4_eae02b66b_p1: tick 3496: 0 (0x00000000) vs -39.838218688964844 (0xC21F5A56)
         corpus_v3_ys_iceclimbers_fox4_e1895a33d_p1: tick 250: 0 (0x00000000) vs 9.264213562011719 (0x41143A38)
         slp_fd_iceclimbers_falco_r195613_t5900: tick 5714: 0 (0x00000000) vs -65.66716003417969 (0xC2835596)
    7  particles.link[].particle[].position[] | retail bank 0 program 454 | port bank 0 program 454
         corpus_v3_fd_ness_fox4_e7b868c1e_p1: tick 832: 11.14710807800293 (0x41325A8E) vs 11.147106170654297 (0x41325A8C)
         corpus_v3_fd_ness_fox4_ef89b3e70_p0: tick 305: 0.3373388350009918 (0x3EACB7AD) vs 0.33733874559402466 (0x3EACB7AA)
         ness_pkflash_blast_fd_fox4: tick 329: 1.043823003768921 (0x3F859BFE) vs 1.0438228845596313 (0x3F859BFD)
         ness_pkflash_fd_fox4: tick 389: 11.568330764770508 (0x413917E2) vs 11.568328857421875 (0x413917E0)
         ness_pkflash_hit_fd_fox4: tick 319: -3.66948914527893 (0xC06AD8E9) vs -3.6694884300231934 (0xC06AD8E6)
         ness_pkflash_shield_fd_fox4: tick 329: 4.1331610679626465 (0x408442DB) vs 4.133160591125488 (0x408442DA)
         pair_light_hit_captor_launched_pkfire_fd_ness_fox4: tick 832: 11.14710807800293 (0x41325A8E) vs 11.147106170654297 (0x41325A8C)
    6  particles.generator[].position[] | retail bank 0 program 93 | port bank 0 program 93
         corpus_v3_dl_fox_marth4_ebe7703ed_p0: tick 4036: -4.604039192199707 (0xC093544A) vs -4.604040622711182 (0xC093544D)
         corpus_v3_fd_bowser_fox4_ec13743d5_p0: tick 806: 30.172468185424805 (0x41F16137) vs 30.172466278076172 (0x41F16136)
         corpus_v3_fd_bowser_fox4_ef6b5a67f_p0: tick 5655: 7.493805885314941 (0x40EFCD42) vs 7.493804931640625 (0x40EFCD40)
         corpus_v3_fd_captainfalcon_fox4_e45a17231_p0: tick 956: 5.946535587310791 (0x40BE4A05) vs 5.946537494659424 (0x40BE4A09)
         corpus_v3_fd_jigglypuff_fox4_e8be4d273_p0: tick 357: -52.040645599365234 (0xC250299F) vs -52.0406494140625 (0xC25029A0)
         corpus_v3_fd_luigi_fox4_e121faf54_p2: tick 904: 56.24863815307617 (0x4260FE9B) vs 56.24864196777344 (0x4260FE9C)
    6  particles.generator[].position[] | retail bank 0 program 64 | port bank 0 program 64
         corpus_v3_fd_link_fox4_ef5188d7f_p2: tick 1703: -46.65977478027344 (0xC23AA39C) vs -49.192657470703125 (0xC244C548)
         corpus_v3_fd_ness_fox4_e05ae623b_p0: tick 809: 7.3532257080078125 (0x40EB4DA0) vs 2.705310821533203 (0x402D23D0)
         corpus_v3_fd_ness_fox4_ebb526384_p0: tick 839: -27.13286018371582 (0xC1D91019) vs -23.136070251464844 (0xC1B916AC)
         corpus_v3_fd_ness_fox4_ec3145eb3_p2: tick 188: -54.68463134765625 (0xC25ABD10) vs -50.83372116088867 (0xC24B55BB)
         corpus_v3_fd_younglink_fox4_eeda0d0fc_p1: tick 532: -42.29377746582031 (0xC2292CD4) vs -44.416786193847656 (0xC231AACA)
         ness_yoyo_charge_struck_fd_fox4: tick 184: -59.74847412109375 (0xC26EFE70) vs -64.75321960449219 (0xC28181A6)
    5  particles.generator[].position[] | retail bank 0 program 294 | port bank 0 program 294
         corpus_v3_fd_ness_fox4_e336c503c_p1: tick 3485: 0 (0x00000000) vs -27.05704689025879 (0xC1D874D5)
         corpus_v3_fd_ness_fox4_ebe7703ed_p0: tick 4094: 0 (0x00000000) vs 38.66647720336914 (0x421AAA79)
         corpus_v3_fd_ness_fox4_ee98155f3_p0: tick 366: 0 (0x00000000) vs 14.362573623657227 (0x4165CD1A)
         ness_pkfire_hit_fd_fox4: tick 320: 0 (0x00000000) vs 56.18193054199219 (0x4260BA4C)
         ness_pkfire_reflect_fd_fox4: tick 327: 0 (0x00000000) vs 34.22536087036133 (0x4208E6C5)
    4  NOT COMPARED dump not compared: its pass wrote N records of M
         corpus_v3_ps_iceclimbers_peach4_ec3145eb3_p2_edgeguard: dump not compared: its pass wrote 5278 records of 6001
         slp_bf_drmario_fox_t8628: dump not compared: its pass wrote 4919 records of 8121
         slp_ps_drmario_peach_fzw_t9200: dump not compared: its pass wrote 7420 records of 9200
         slp_ps_fox_drmario_wide9263_lag9382_t9800: dump not compared: its pass wrote 8846 records of 9800
    3  particles.generator[].position[] | retail bank 0 program 270 | port bank 0 program 270
         corpus_v2_s0_e2a_p1: tick 280: -25.363327026367188 (0xC1CAE818) vs -25.363300323486328 (0xC1CAE80A)
         corpus_v2_s1_e1_p0: tick 286: 46.997703552246094 (0x423BFDA6) vs 46.99604034423828 (0x423BFBF2)
         corpus_v2_s1_edeadbeef_p1: tick 194: -38.24503707885742 (0xC218FAEB) vs -38.24502944946289 (0xC218FAE9)
    2  particles.generator[].position[] | retail bank 14 program 14000 | port bank 14 program 14000
         corpus_v3_bf_iceclimbers_fox4_e25aa37fe_p1: tick 1275: 0 (0x00000000) vs 9.52514362335205 (0x411866FD)
         slp_fd_captainfalcon_iceclimbers_r001727_t17300: tick 161: 0 (0x00000000) vs 19.359970092773438 (0x419AE138)
    2  particles.generator[].position[] | retail bank 0 program 225 | port bank 0 program 225
         corpus_v3_s1_e37053c42_p0: tick 2419: -79.41654968261719 (0xC29ED546) vs -79.41706848144531 (0xC29ED58A)
         slp_fd_captainfalcon_samus_t10400: tick 1853: 6.890323162078857 (0x40DC7D87) vs 6.890324115753174 (0x40DC7D89)
    1  particles.generator[].position[] | retail bank 0 program 74 | port bank 0 program 74
         corpus_v3_bf_yoshi_pikachu4_ec13743d5_p2: tick 918: 55.240020751953125 (0x425CF5C8) vs 49.678802490234375 (0x4246B718)
    1  particles.generator[].position[] | retail bank 0 program 408 | port bank 0 program 408
         corpus_v3_fd_ganondorf_fox4_e0ac13e8e_p2: tick 1580: -69.6690902709961 (0xC28B5693) vs -70.10433959960938 (0xC28C356C)
    1  particles.generator[].position[] | retail bank 0 program 8 | port bank 0 program 8
         corpus_v3_ps_samus_fox4_e51dc0146_p0: tick 3242: 1.097318172454834 (0x3F8C74EC) vs 1.197318196296692 (0x3F9941B9)
    1  particles.generator[].position[] | retail bank 0 program 266 | port bank 0 program 266
         corpus_v3_ys_peach_yoshi4_e89a89d0e_p1: tick 1032: -45.7714958190918 (0xC2371603) vs -45.76953125 (0xC2371400)
    1  particles.generator[].position[] | retail bank 0 program 28 | port bank 0 program 28
         mewtwo_ball_burst_fd_fox4: tick 202: -86.31795501708984 (0xC2ACA2CB) vs -89.50608825683594 (0xC2B3031E)
    1  particles.link[].particle[].position[] | retail bank 0 program 455 | port bank 0 program 455
         ness_pkflash_struck_fd_fox4: tick 204: -11.839747428894045 (0xC13D6F9B) vs -11.839746475219727 (0xC13D6F9A)
    1  particles.generator[].position[] | retail bank 0 program 91 | port bank 0 program 91
         peach_airdodge_turnip_catch_fd_fox4: tick 274: -63.01520919799805 (0xC27C0F93) vs -62.622562408447266 (0xC27A7D81)
    1  particles.generator[].position[] | retail bank 0 program 107 | port bank 0 program 107
         slp_fd_samus_pikachu_t4250: tick 2194: 30.240398406982425 (0x41F1EC56) vs 35.48066329956055 (0x420DEC33)
    1  particles.generator[].position[] | retail bank 0 program 108 | port bank 0 program 108
         slp_ps_iceclimbers_samus_r191618_t6750: tick 6609: -136.19264221191406 (0xC3083151) vs -138.9342498779297 (0xC30AEF2B)
    1  particles.link[].particle[].position[] | retail bank 0 program 19 | port bank 0 program 19
         slp_ys_fox_falco_t1900: tick 353: -5.903419494628906 (0xC0BCE8D0) vs -5.9034199714660645 (0xC0BCE8D1)

```
