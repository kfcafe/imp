# Stale docs audit

## Version references
Cargo.toml:37:version = "0.3.0"
Cargo.toml:50:async-trait = "0.1"
Cargo.toml:53:futures-core = "0.3"
Cargo.toml:54:futures = "0.3"
Cargo.toml:56:wait-timeout = "0.2"
Cargo.toml:57:reqwest = { version = "0.12", features = ["stream", "json"] }
Cargo.toml:58:tokio-tungstenite = { version = "0.27", features = ["native-tls"] }
Cargo.toml:59:tungstenite = "0.27"
Cargo.toml:64:ratatui = "0.29"
Cargo.toml:65:crossterm = "0.28"
Cargo.toml:66:eframe = "0.31"
Cargo.toml:67:egui = "0.31"
Cargo.toml:68:mlua = { version = "0.10", features = ["lua54", "vendored", "async", "send"] }
Cargo.toml:72:tree-sitter = "0.26"
Cargo.toml:73:tree-sitter-rust = "0.24"
Cargo.toml:74:tree-sitter-typescript = "0.23"
Cargo.toml:75:tree-sitter-python = "0.25"
Cargo.toml:76:tree-sitter-go = "0.25"
Cargo.toml:78:tree-sitter-bash = "0.25"
Cargo.toml:79:tree-sitter-elixir = "0.3"
Cargo.toml:80:tree-sitter-ruby = "0.23"
Cargo.toml:81:tree-sitter-ocaml = "0.25"
Cargo.toml:87:tree-sitter-java = "0.23"
Cargo.toml:88:tree-sitter-c = "0.24"
Cargo.toml:89:tree-sitter-cpp = "0.23"
Cargo.toml:90:tree-sitter-c-sharp = "0.23"
Cargo.toml:91:tree-sitter-javascript = "0.25"
Cargo.toml:92:tree-sitter-php = "0.24"
Cargo.toml:93:tree-sitter-scala = "0.26"
Cargo.toml:94:tree-sitter-dart = "0.2"
Cargo.toml:95:readability-rust = "0.1"
Cargo.toml:96:project-detect = "0.1"
Cargo.toml:97:mana-core = "0.3.2"
Cargo.toml:99:imp-llm = { version = "0.3.0", path = "crates/imp-llm" }
Cargo.toml:100:imp-core = { version = "0.3.0", path = "crates/imp-core" }
Cargo.toml:101:imp-lua = { version = "0.3.0", path = "crates/imp-lua" }
Cargo.toml:102:imp-tui = { version = "0.3.0", path = "crates/imp-tui" }
Cargo.toml:103:imp-gui = { version = "0.3.0", path = "crates/imp-gui" }
Cargo.toml:104:imp-cli = { version = "0.3.0", path = "crates/imp-cli" }
Cargo.lock:7:version = "0.2.32"
Cargo.lock:17:version = "0.1.10"
Cargo.lock:23:version = "0.17.1"
Cargo.lock:29:version = "0.10.1"
Cargo.lock:43:version = "0.26.0"
Cargo.lock:48: "hashbrown 0.15.5",
Cargo.lock:54:version = "0.18.1"
Cargo.lock:60: "hashbrown 0.15.5",
Cargo.lock:61: "objc2 0.5.2",
Cargo.lock:62: "objc2-app-kit 0.2.2",
Cargo.lock:63: "objc2-foundation 0.2.2",
Cargo.lock:68:version = "0.13.1"
Cargo.lock:86:version = "0.24.1"
Cargo.lock:92: "hashbrown 0.15.5",
Cargo.lock:95: "windows 0.58.0",
Cargo.lock:96: "windows-core 0.58.0",
Cargo.lock:101:version = "0.23.1"
Cargo.lock:115:version = "2.0.1"
Cargo.lock:121:version = "0.8.4"
Cargo.lock:132:version = "0.8.12"
Cargo.lock:137: "getrandom 0.3.4",
Cargo.lock:155:version = "0.2.21"
Cargo.lock:161:version = "0.6.1"
Cargo.lock:173: "ndk-sys 0.6.0+11769913",
Cargo.lock:175: "thiserror 2.0.18",
Cargo.lock:180:version = "0.2.2"
Cargo.lock:186:version = "0.1.5"
Cargo.lock:210:version = "1.0.14"
Cargo.lock:229: "windows-sys 0.61.2",
Cargo.lock:234:version = "3.0.11"
Cargo.lock:240: "windows-sys 0.61.2",
Cargo.lock:245:version = "1.0.102"
Cargo.lock:258: "objc2 0.6.4",
Cargo.lock:259: "objc2-app-kit 0.3.2",
Cargo.lock:262: "objc2-foundation 0.3.2",
Cargo.lock:265: "windows-sys 0.60.2",
Cargo.lock:271:version = "0.3.9"
Cargo.lock:277:version = "0.7.6"
Cargo.lock:283:version = "1.0.1"
Cargo.lock:289:version = "0.38.0+1.3.281"
Cargo.lock:298:version = "0.7.2"
Cargo.lock:360: "windows-sys 0.61.2",
Cargo.lock:400: "syn 2.0.117",
Cargo.lock:405:version = "0.2.14"
Cargo.lock:418: "windows-sys 0.61.2",
Cargo.lock:429:version = "0.1.89"
Cargo.lock:435: "syn 2.0.117",
Cargo.lock:446:version = "0.22.0"
Cargo.lock:457:version = "0.6.0"
Cargo.lock:473:version = "0.6.0"
Cargo.lock:485:version = "0.6.0"
Cargo.lock:513:version = "0.41.0"
Cargo.lock:525:version = "0.22.1"
Cargo.lock:540:version = "0.8.0"
Cargo.lock:549:version = "0.8.0"
Cargo.lock:570:version = "0.1.6"
Cargo.lock:576:version = "0.10.4"
Cargo.lock:585:version = "0.3.3"
Cargo.lock:594:version = "0.5.1"
Cargo.lock:598: "objc2 0.5.2",
Cargo.lock:616:version = "0.2.4"
Cargo.lock:632:version = "3.20.3"
Cargo.lock:638:version = "0.6.9"
Cargo.lock:653:version = "1.10.2"
Cargo.lock:659: "syn 2.0.117",
Cargo.lock:670:version = "0.1.0"
Cargo.lock:682:version = "0.13.0"
Cargo.lock:689: "rustix 0.38.44",
Cargo.lock:696:version = "0.14.4"
Cargo.lock:709:version = "0.3.0"
Cargo.lock:713: "calloop 0.13.0",
Cargo.lock:714: "rustix 0.38.44",
Cargo.lock:721:version = "0.4.1"
Cargo.lock:725: "calloop 0.14.4",
Cargo.lock:733:version = "0.3.0"
Cargo.lock:739:version = "0.2.4"
Cargo.lock:748:version = "0.1.2"
Cargo.lock:775:version = "0.2.1"
Cargo.lock:781:version = "0.3.2"
Cargo.lock:790:version = "0.4.44"
Cargo.lock:804:version = "0.4.4"
Cargo.lock:852: "syn 2.0.117",
Cargo.lock:872:version = "0.1.58"
Cargo.lock:881:version = "0.11.1"
Cargo.lock:886: "unicode-width 0.1.14",
Cargo.lock:907:version = "0.8.1"
Cargo.lock:930:version = "0.16.3"
Cargo.lock:936: "unicode-width 0.2.0",
Cargo.lock:937: "windows-sys 0.61.2",
Cargo.lock:942:version = "0.10.0"
Cargo.lock:955: "crossterm 0.29.0",
Cargo.lock:960:version = "0.9.4"
Cargo.lock:970:version = "0.10.1"
Cargo.lock:980:version = "0.8.7"
Cargo.lock:986:version = "0.23.2"
Cargo.lock:991: "core-foundation 0.9.4",
Cargo.lock:993: "foreign-types 0.5.0",
Cargo.lock:999:version = "0.1.3"
Cargo.lock:1004: "core-foundation 0.9.4",
Cargo.lock:1010:version = "0.2.17"
Cargo.lock:1033: "crossterm 0.29.0",
Cargo.lock:1045: "crossterm 0.29.0",
Cargo.lock:1049: "syn 2.0.117",
Cargo.lock:1054:version = "0.8.4"
Cargo.lock:1067:version = "0.5.15"
Cargo.lock:1076:version = "0.8.6"
Cargo.lock:1086:version = "0.9.18"
Cargo.lock:1095:version = "0.3.12"
Cargo.lock:1104:version = "0.8.21"
Cargo.lock:1110:version = "0.28.1"
Cargo.lock:1118: "rustix 0.38.44",
Cargo.lock:1126:version = "0.29.0"
Cargo.lock:1144:version = "0.9.1"
Cargo.lock:1153:version = "0.2.4"
Cargo.lock:1159:version = "0.1.7"
Cargo.lock:1169:version = "0.31.2"
Cargo.lock:1176: "phf 0.11.3",
Cargo.lock:1182:version = "0.6.1"
Cargo.lock:1187: "syn 2.0.117",
Cargo.lock:1198:version = "0.23.0"
Cargo.lock:1208:version = "0.23.0"
Cargo.lock:1216: "syn 2.0.117",
Cargo.lock:1221:version = "0.23.0"
Cargo.lock:1227: "syn 2.0.117",
Cargo.lock:1238:version = "0.9.11"
Cargo.lock:1244: "windows-sys 0.61.2",
Cargo.lock:1267:version = "0.5.8"
Cargo.lock:1276:version = "0.99.20"
Cargo.lock:1282: "syn 2.0.117",
Cargo.lock:1304: "syn 2.0.117",
Cargo.lock:1309:version = "0.12.0"
Cargo.lock:1322:version = "0.10.7"
Cargo.lock:1342:version = "0.5.0"
Cargo.lock:1349: "windows-sys 0.61.2",
Cargo.lock:1354:version = "0.2.0"
Cargo.lock:1360:version = "0.3.1"
Cargo.lock:1365: "objc2 0.6.4",
Cargo.lock:1370:version = "0.2.5"
Cargo.lock:1376: "syn 2.0.117",
Cargo.lock:1381:version = "0.5.3"
Cargo.lock:1390:version = "0.2.12"
Cargo.lock:1405:version = "0.1.2"
Cargo.lock:1411:version = "1.0.11"
Cargo.lock:1417:version = "0.3.5"
Cargo.lock:1432:version = "0.31.1"
Cargo.lock:1442:version = "0.31.1"
Cargo.lock:1459: "objc2 0.5.2",
Cargo.lock:1460: "objc2-app-kit 0.2.2",
Cargo.lock:1461: "objc2-foundation 0.2.2",
Cargo.lock:1472: "windows-sys 0.59.0",
Cargo.lock:1478:version = "0.6.3"
Cargo.lock:1484:version = "0.31.1"
Cargo.lock:1500:version = "0.31.1"
Cargo.lock:1520:version = "0.31.1"
Cargo.lock:1540:version = "0.31.1"
Cargo.lock:1564:version = "0.2.9"
Cargo.lock:1573:version = "0.31.1"
Cargo.lock:1588:version = "0.8.35"
Cargo.lock:1603:version = "0.7.12"
Cargo.lock:1613:version = "0.7.12"
Cargo.lock:1619: "syn 2.0.117",
Cargo.lock:1624:version = "0.1.0"
Cargo.lock:1630:version = "0.31.1"
Cargo.lock:1648:version = "0.31.1"
Cargo.lock:1654:version = "1.0.2"
Cargo.lock:1660:version = "0.3.14"
Cargo.lock:1665: "windows-sys 0.61.2",
Cargo.lock:1687:version = "0.5.4"
Cargo.lock:1697:version = "0.3.0"
Cargo.lock:1703:version = "0.1.9"
Cargo.lock:1709:version = "0.17.0"
Cargo.lock:1726:version = "0.2.7"
Cargo.lock:1732:version = "0.3.7"
Cargo.lock:1741:version = "0.1.9"
Cargo.lock:1757:version = "0.4.1"
Cargo.lock:1774:version = "0.1.5"
Cargo.lock:1780:version = "0.2.0"
Cargo.lock:1786:version = "0.3.2"
Cargo.lock:1790: "foreign-types-shared 0.1.1",
Cargo.lock:1795:version = "0.5.0"
Cargo.lock:1800: "foreign-types-shared 0.3.1",
Cargo.lock:1805:version = "0.2.3"
Cargo.lock:1811: "syn 2.0.117",
Cargo.lock:1816:version = "0.1.1"
Cargo.lock:1822:version = "0.3.1"
Cargo.lock:1837:version = "0.15.4"
Cargo.lock:1847:version = "0.4.3"
Cargo.lock:1863:version = "0.1.5"
Cargo.lock:1873:version = "0.3.32"
Cargo.lock:1888:version = "0.3.32"
Cargo.lock:1898:version = "0.3.32"
Cargo.lock:1904:version = "0.3.32"
Cargo.lock:1915:version = "0.3.32"
Cargo.lock:1934:version = "0.3.32"
Cargo.lock:1940: "syn 2.0.117",
Cargo.lock:1945:version = "0.3.32"
Cargo.lock:1951:version = "0.3.32"
Cargo.lock:1957:version = "0.3.32"
Cargo.lock:1974:version = "0.3.7"
Cargo.lock:1983:version = "0.2.1"
Cargo.lock:1992:version = "0.14.7"
Cargo.lock:2012:version = "0.2.24"
Cargo.lock:2016: "unicode-width 0.2.0",
Cargo.lock:2021:version = "0.2.17"
Cargo.lock:2032:version = "0.3.4"
Cargo.lock:2046:version = "0.4.2"
Cargo.lock:2059:version = "0.14.0"
Cargo.lock:2070:version = "0.3.3"
Cargo.lock:2076:version = "0.4.18"
Cargo.lock:2089:version = "0.16.0"
Cargo.lock:2101:version = "0.32.3"
Cargo.lock:2113: "objc2 0.6.4",
Cargo.lock:2114: "objc2-app-kit 0.3.2",
Cargo.lock:2116: "objc2-foundation 0.3.2",
Cargo.lock:2120: "windows-sys 0.52.0",
Cargo.lock:2126:version = "0.5.0"
Cargo.lock:2138:version = "0.7.1"
Cargo.lock:2143: "windows-sys 0.52.0",
Cargo.lock:2148:version = "0.6.1"
Cargo.lock:2158:version = "0.6.1"
Cargo.lock:2167:version = "0.6.0"
Cargo.lock:2177:version = "0.3.0"
Cargo.lock:2186:version = "0.3.2"
Cargo.lock:2192: "hashbrown 0.15.5",
Cargo.lock:2197:version = "0.2.0"
Cargo.lock:2206:version = "0.4.14"
Cargo.lock:2236:version = "0.15.5"
Cargo.lock:2242: "foldhash 0.1.5",
Cargo.lock:2247:version = "0.16.1"
Cargo.lock:2253: "foldhash 0.2.0",
Cargo.lock:2258:version = "0.17.1"
Cargo.lock:2264:version = "0.11.0"
Cargo.lock:2268: "hashbrown 0.16.1",
Cargo.lock:2273:version = "0.5.0"
Cargo.lock:2279:version = "0.5.2"
Cargo.lock:2285:version = "0.4.3"
Cargo.lock:2291:version = "0.2.1"
Cargo.lock:2297:version = "0.12.4"
Cargo.lock:2306:version = "0.12.1"
Cargo.lock:2315:version = "0.26.0"
Cargo.lock:2324: "syn 1.0.109",
Cargo.lock:2339:version = "1.0.1"
Cargo.lock:2349:version = "0.1.3"
Cargo.lock:2362:version = "1.10.1"
Cargo.lock:2389:version = "0.27.9"
Cargo.lock:2404:version = "0.6.0"
Cargo.lock:2420:version = "0.1.20"
Cargo.lock:2445:version = "0.1.65"
Cargo.lock:2455: "windows-core 0.62.2",
Cargo.lock:2460:version = "0.1.2"
Cargo.lock:2557:version = "1.0.1"
Cargo.lock:2584:version = "0.4.25"
Cargo.lock:2600:version = "0.25.10"
Cargo.lock:2623:version = "0.3.0"
Cargo.lock:2643:version = "0.3.0"
Cargo.lock:2661: "reqwest 0.12.28",
Cargo.lock:2668: "thiserror 2.0.18",
Cargo.lock:2701:version = "0.3.0"
Cargo.lock:2709: "thiserror 2.0.18",
Cargo.lock:2714:version = "0.3.0"
Cargo.lock:2722:version = "0.3.0"
Cargo.lock:2729: "rand 0.8.6",
Cargo.lock:2730: "reqwest 0.12.28",
Cargo.lock:2735: "thiserror 2.0.18",
Cargo.lock:2744:version = "0.3.0"
Cargo.lock:2751: "reqwest 0.12.28",
Cargo.lock:2754: "thiserror 2.0.18",
Cargo.lock:2760:version = "0.3.0"
Cargo.lock:2763: "crossterm 0.28.1",
Cargo.lock:2776: "thiserror 2.0.18",
Cargo.lock:2778: "unicode-width 0.2.0",
Cargo.lock:2790: "hashbrown 0.17.1",
Cargo.lock:2806:version = "0.1.4"
Cargo.lock:2816:version = "0.3.12"
Cargo.lock:2824: "syn 2.0.117",
Cargo.lock:2835:version = "1.70.2"
Cargo.lock:2841:version = "0.13.0"
Cargo.lock:2850:version = "1.0.18"
Cargo.lock:2856:version = "0.22.4"
Cargo.lock:2863: "jni-sys 0.4.1",
Cargo.lock:2866: "thiserror 2.0.18",
Cargo.lock:2873:version = "0.22.4"
Cargo.lock:2881: "syn 2.0.117",
Cargo.lock:2886:version = "0.3.1"
Cargo.lock:2890: "jni-sys 0.4.1",
Cargo.lock:2895:version = "0.4.1"
Cargo.lock:2904:version = "0.4.1"
Cargo.lock:2909: "syn 2.0.117",
Cargo.lock:2914:version = "0.1.34"
Cargo.lock:2918: "getrandom 0.3.4",
Cargo.lock:2924:version = "0.3.99"
Cargo.lock:2936:version = "0.45.1"
Cargo.lock:2946: "getrandom 0.3.4",
Cargo.lock:2955: "reqwest 0.13.4",
Cargo.lock:2975: "windows-sys 0.60.2",
Cargo.lock:3016: "syn 2.0.117",
Cargo.lock:3027:version = "0.1.0"
Cargo.lock:3033:version = "0.2.186"
Cargo.lock:3039:version = "0.2.7"
Cargo.lock:3048:version = "0.8.9"
Cargo.lock:3058:version = "0.1.16"
Cargo.lock:3065: "redox_syscall 0.7.5",
Cargo.lock:3070:version = "0.37.0"
Cargo.lock:3081:version = "0.0.5"
Cargo.lock:3091:version = "0.5.6"
Cargo.lock:3097:version = "0.4.15"
Cargo.lock:3103:version = "0.12.1"
Cargo.lock:3109:version = "0.8.2"
Cargo.lock:3121:version = "0.4.14"
Cargo.lock:3130:version = "0.4.30"
Cargo.lock:3136:version = "0.12.5"
Cargo.lock:3140: "hashbrown 0.15.5",
Cargo.lock:3154:version = "210.5.12+a4f56a4"
Cargo.lock:3164:version = "0.1.1"
Cargo.lock:3170:version = "0.0.6"
Cargo.lock:3179:version = "0.3.2"
Cargo.lock:3207:version = "0.3.2"
Cargo.lock:3224: "thiserror 2.0.18",
Cargo.lock:3229:version = "0.1.1"
Cargo.lock:3241:version = "0.11.0"
Cargo.lock:3246: "phf 0.10.1",
Cargo.lock:3261:version = "0.9.10"
Cargo.lock:3270:version = "0.9.1"
Cargo.lock:3279:version = "0.31.0"
Cargo.lock:3286: "foreign-types 0.5.0",
Cargo.lock:3294:version = "0.3.17"
Cargo.lock:3300:version = "0.14.0"
Cargo.lock:3309:version = "0.8.9"
Cargo.lock:3326: "windows-sys 0.61.2",
Cargo.lock:3331:version = "0.10.5"
Cargo.lock:3347:version = "0.6.8"
Cargo.lock:3360:version = "0.8.1"
Cargo.lock:3386: "thiserror 2.0.18",
Cargo.lock:3392:version = "0.2.18"
Cargo.lock:3409:version = "0.9.0"
Cargo.lock:3414: "jni-sys 0.3.1",
Cargo.lock:3416: "ndk-sys 0.6.0+11769913",
Cargo.lock:3424:version = "0.1.1"
Cargo.lock:3430:version = "0.5.0+25.2.9519653"
Cargo.lock:3434: "jni-sys 0.3.1",
Cargo.lock:3439:version = "0.6.0+11769913"
Cargo.lock:3443: "jni-sys 0.3.1",
Cargo.lock:3454:version = "0.29.0"
Cargo.lock:3467:version = "0.2.0"
Cargo.lock:3473:version = "0.4.3"
Cargo.lock:3482:version = "0.4.3"
Cargo.lock:3496:version = "0.4.6"
Cargo.lock:3506:version = "0.1.0"
Cargo.lock:3512:version = "0.4.6"
Cargo.lock:3521:version = "0.2.2"
Cargo.lock:3527:version = "0.1.46"
Cargo.lock:3536:version = "0.1.45"
Cargo.lock:3547:version = "0.4.2"
Cargo.lock:3558:version = "0.2.19"
Cargo.lock:3567:version = "0.7.6"
Cargo.lock:3577:version = "0.7.6"
Cargo.lock:3584: "syn 2.0.117",
Cargo.lock:3589:version = "0.2.7"
Cargo.lock:3598:version = "0.3.5"
Cargo.lock:3604:version = "0.5.2"
Cargo.lock:3614:version = "0.6.4"
Cargo.lock:3623:version = "0.2.2"
Cargo.lock:3630: "objc2 0.5.2",
Cargo.lock:3633: "objc2-foundation 0.2.2",
Cargo.lock:3639:version = "0.3.2"
Cargo.lock:3644: "objc2 0.6.4",
Cargo.lock:3647: "objc2-foundation 0.3.2",
Cargo.lock:3652:version = "0.2.2"
Cargo.lock:3658: "objc2 0.5.2",
Cargo.lock:3660: "objc2-foundation 0.2.2",
Cargo.lock:3665:version = "0.2.2"
Cargo.lock:3670: "objc2 0.5.2",
Cargo.lock:3671: "objc2-foundation 0.2.2",
Cargo.lock:3676:version = "0.2.2"
Cargo.lock:3682: "objc2 0.5.2",
Cargo.lock:3683: "objc2-foundation 0.2.2",
Cargo.lock:3688:version = "0.3.2"
Cargo.lock:3694: "objc2 0.6.4",
Cargo.lock:3699:version = "0.3.2"
Cargo.lock:3705: "objc2 0.6.4",
Cargo.lock:3712:version = "0.2.2"
Cargo.lock:3717: "objc2 0.5.2",
Cargo.lock:3718: "objc2-foundation 0.2.2",
Cargo.lock:3724:version = "0.2.2"
Cargo.lock:3729: "objc2 0.5.2",
Cargo.lock:3731: "objc2-foundation 0.2.2",
Cargo.lock:3742:version = "0.2.2"
Cargo.lock:3750: "objc2 0.5.2",
Cargo.lock:3755:version = "0.3.2"
Cargo.lock:3760: "objc2 0.6.4",
Cargo.lock:3766:version = "0.3.2"
Cargo.lock:3771: "objc2 0.6.4",
Cargo.lock:3777:version = "0.2.2"
Cargo.lock:3782: "objc2 0.5.2",
Cargo.lock:3783: "objc2-app-kit 0.2.2",
Cargo.lock:3784: "objc2-foundation 0.2.2",
Cargo.lock:3789:version = "0.2.2"
Cargo.lock:3795: "objc2 0.5.2",
Cargo.lock:3796: "objc2-foundation 0.2.2",
Cargo.lock:3801:version = "0.2.2"
Cargo.lock:3807: "objc2 0.5.2",
Cargo.lock:3808: "objc2-foundation 0.2.2",
Cargo.lock:3814:version = "0.2.2"
Cargo.lock:3818: "objc2 0.5.2",
Cargo.lock:3819: "objc2-foundation 0.2.2",
Cargo.lock:3824:version = "0.2.2"
Cargo.lock:3830: "objc2 0.5.2",
Cargo.lock:3835: "objc2-foundation 0.2.2",
Cargo.lock:3845:version = "0.2.2"
Cargo.lock:3850: "objc2 0.5.2",
Cargo.lock:3851: "objc2-foundation 0.2.2",
Cargo.lock:3856:version = "0.2.2"
Cargo.lock:3862: "objc2 0.5.2",
Cargo.lock:3864: "objc2-foundation 0.2.2",
Cargo.lock:3875:version = "1.70.2"
Cargo.lock:3903:version = "0.10.80"
Cargo.lock:3909: "foreign-types 0.3.2",
Cargo.lock:3917:version = "0.1.1"
Cargo.lock:3923: "syn 2.0.117",
Cargo.lock:3928:version = "0.2.1"
Cargo.lock:3934:version = "0.9.116"
Cargo.lock:3946:version = "0.2.0"
Cargo.lock:3952:version = "0.3.55"
Cargo.lock:3971:version = "0.2.0"
Cargo.lock:3981:version = "0.5.2"
Cargo.lock:3987:version = "0.25.1"
Cargo.lock:4002:version = "0.12.5"
Cargo.lock:4012:version = "0.9.12"
Cargo.lock:4018: "redox_syscall 0.5.18",
Cargo.lock:4025:version = "1.0.15"
Cargo.lock:4031:version = "0.2.3"
Cargo.lock:4043:version = "0.10.1"
Cargo.lock:4047: "phf_shared 0.10.0",
Cargo.lock:4052:version = "0.11.3"
Cargo.lock:4057: "phf_shared 0.11.3",
Cargo.lock:4062:version = "0.10.0"
Cargo.lock:4066: "phf_generator 0.10.0",
Cargo.lock:4067: "phf_shared 0.10.0",
Cargo.lock:4072:version = "0.10.0"
Cargo.lock:4076: "phf_shared 0.10.0",
Cargo.lock:4077: "rand 0.8.6",
Cargo.lock:4082:version = "0.11.3"
Cargo.lock:4086: "phf_shared 0.11.3",
Cargo.lock:4087: "rand 0.8.6",
Cargo.lock:4092:version = "0.11.3"
Cargo.lock:4096: "phf_generator 0.11.3",
Cargo.lock:4097: "phf_shared 0.11.3",
Cargo.lock:4100: "syn 2.0.117",
Cargo.lock:4105:version = "0.10.0"
Cargo.lock:4109: "siphasher 0.3.11",
Cargo.lock:4114:version = "0.11.3"
Cargo.lock:4118: "siphasher 1.0.3",
Cargo.lock:4138: "syn 2.0.117",
Cargo.lock:4143:version = "0.2.17"
Cargo.lock:4149:version = "0.2.5"
Cargo.lock:4160:version = "0.3.33"
Cargo.lock:4166:version = "0.2.3"
Cargo.lock:4178: "quick-xml 0.39.4",
Cargo.lock:4185:version = "0.18.1"
Cargo.lock:4207: "windows-sys 0.61.2",
Cargo.lock:4212:version = "0.1.5"
Cargo.lock:4221:version = "0.2.0"
Cargo.lock:4227:version = "0.2.21"
Cargo.lock:4236:version = "0.1.1"
Cargo.lock:4242:version = "0.2.37"
Cargo.lock:4247: "syn 2.0.117",
Cargo.lock:4256: "toml_edit 0.25.11+spec-1.1.0",
Cargo.lock:4261:version = "1.0.106"
Cargo.lock:4270:version = "1.0.18"
Cargo.lock:4276:version = "0.1.2"
Cargo.lock:4286:version = "0.1.29"
Cargo.lock:4292:version = "2.0.1"
Cargo.lock:4298:version = "0.30.0"
Cargo.lock:4308:version = "0.39.4"
Cargo.lock:4338:version = "0.8.6"
Cargo.lock:4343: "rand_chacha 0.3.1",
Cargo.lock:4344: "rand_core 0.6.4",
Cargo.lock:4349:version = "0.9.4"
Cargo.lock:4353: "rand_chacha 0.9.0",
Cargo.lock:4354: "rand_core 0.9.5",
Cargo.lock:4359:version = "0.3.1"
Cargo.lock:4364: "rand_core 0.6.4",
Cargo.lock:4369:version = "0.9.0"
Cargo.lock:4374: "rand_core 0.9.5",
Cargo.lock:4379:version = "0.6.4"
Cargo.lock:4383: "getrandom 0.2.17",
Cargo.lock:4388:version = "0.9.5"
Cargo.lock:4392: "getrandom 0.3.4",
Cargo.lock:4397:version = "0.29.0"
Cargo.lock:4404: "crossterm 0.28.1",
Cargo.lock:4413: "unicode-width 0.2.0",
Cargo.lock:4418:version = "0.6.2"
Cargo.lock:4444:version = "0.1.0"
Cargo.lock:4461:version = "0.4.1"
Cargo.lock:4470:version = "0.5.18"
Cargo.lock:4479:version = "0.7.5"
Cargo.lock:4488:version = "0.5.2"
Cargo.lock:4492: "getrandom 0.2.17",
Cargo.lock:4494: "thiserror 2.0.18",
Cargo.lock:4499:version = "1.0.25"
Cargo.lock:4508:version = "1.0.25"
Cargo.lock:4514: "syn 2.0.117",
Cargo.lock:4519:version = "0.45.1"
Cargo.lock:4525: "getrandom 0.3.4",
Cargo.lock:4526: "hashbrown 0.16.1",
Cargo.lock:4546:version = "0.4.14"
Cargo.lock:4557:version = "0.8.10"
Cargo.lock:4569:version = "0.12.28"
Cargo.lock:4612:version = "0.13.4"
Cargo.lock:4651:version = "0.17.14"
Cargo.lock:4657: "getrandom 0.2.17",
Cargo.lock:4660: "windows-sys 0.52.0",
Cargo.lock:4665:version = "0.1.1"
Cargo.lock:4669: "hashbrown 0.16.1",
Cargo.lock:4670: "thiserror 2.0.18",
Cargo.lock:4675:version = "0.39.0"
Cargo.lock:4702:version = "0.4.1"
Cargo.lock:4711:version = "0.38.44"
Cargo.lock:4718: "linux-raw-sys 0.4.15",
Cargo.lock:4719: "windows-sys 0.59.0",
Cargo.lock:4731: "linux-raw-sys 0.12.1",
Cargo.lock:4732: "windows-sys 0.61.2",
Cargo.lock:4737:version = "0.23.40"
Cargo.lock:4751:version = "0.8.3"
Cargo.lock:4772:version = "0.7.0"
Cargo.lock:4776: "core-foundation 0.10.1",
Cargo.lock:4788: "windows-sys 0.61.2",
Cargo.lock:4793:version = "0.1.1"
Cargo.lock:4799:version = "0.103.13"
Cargo.lock:4811:version = "1.0.22"
Cargo.lock:4817:version = "1.0.23"
Cargo.lock:4832:version = "0.1.29"
Cargo.lock:4836: "windows-sys 0.61.2",
Cargo.lock:4841:version = "1.0.1"
Cargo.lock:4853:version = "0.18.1"
Cargo.lock:4869:version = "0.10.1"
Cargo.lock:4876: "smithay-client-toolkit 0.19.2",
Cargo.lock:4893: "rand 0.8.6",
Cargo.lock:4906: "core-foundation 0.9.4",
Cargo.lock:4919: "core-foundation 0.10.1",
Cargo.lock:4937:version = "0.25.0"
Cargo.lock:4943: "derive_more 0.99.20",
Cargo.lock:4947: "phf 0.10.1",
Cargo.lock:4956:version = "1.0.28"
Cargo.lock:4962:version = "1.0.228"
Cargo.lock:4972:version = "1.0.228"
Cargo.lock:4981:version = "1.0.228"
Cargo.lock:4987: "syn 2.0.117",
Cargo.lock:4992:version = "1.0.150"
Cargo.lock:5006:version = "0.1.20"
Cargo.lock:5012: "syn 2.0.117",
Cargo.lock:5017:version = "0.6.9"
Cargo.lock:5026:version = "0.7.1"
Cargo.lock:5038:version = "0.9.34+deprecated"
Cargo.lock:5051:version = "0.0.12"
Cargo.lock:5066:version = "0.3.0"
Cargo.lock:5075:version = "0.10.6"
Cargo.lock:5086:version = "0.10.9"
Cargo.lock:5097:version = "0.1.5"
Cargo.lock:5115:version = "0.3.18"
Cargo.lock:5125:version = "0.2.5"
Cargo.lock:5146:version = "0.3.9"
Cargo.lock:5162:version = "0.1.5"
Cargo.lock:5174:version = "0.3.11"
Cargo.lock:5180:version = "1.0.3"
Cargo.lock:5186:version = "0.4.12"
Cargo.lock:5207:version = "0.19.2"
Cargo.lock:5212: "calloop 0.13.0",
Cargo.lock:5213: "calloop-wayland-source 0.3.0",
Cargo.lock:5218: "rustix 0.38.44",
Cargo.lock:5232:version = "0.20.0"
Cargo.lock:5237: "calloop 0.14.4",
Cargo.lock:5238: "calloop-wayland-source 0.4.1",
Cargo.lock:5244: "thiserror 2.0.18",
Cargo.lock:5259:version = "0.7.3"
Cargo.lock:5264: "smithay-client-toolkit 0.20.0",
Cargo.lock:5270:version = "0.2.2"
Cargo.lock:5279:version = "0.6.3"
Cargo.lock:5284: "windows-sys 0.61.2",
Cargo.lock:5289:version = "0.3.0+sdk-1.3.268.0"
Cargo.lock:5298:version = "0.5.5"
Cargo.lock:5322:version = "0.1.9"
Cargo.lock:5328:version = "0.2.0"
Cargo.lock:5334:version = "0.1.1"
Cargo.lock:5340:version = "0.8.9"
Cargo.lock:5346: "phf_shared 0.11.3",
Cargo.lock:5353:version = "0.5.4"
Cargo.lock:5357: "phf_generator 0.11.3",
Cargo.lock:5358: "phf_shared 0.11.3",
Cargo.lock:5365:version = "0.11.1"
Cargo.lock:5371:version = "0.26.3"
Cargo.lock:5380:version = "0.26.4"
Cargo.lock:5388: "syn 2.0.117",
Cargo.lock:5399:version = "1.0.109"
Cargo.lock:5410:version = "2.0.117"
Cargo.lock:5421:version = "1.0.2"
Cargo.lock:5430:version = "0.13.2"
Cargo.lock:5436: "syn 2.0.117",
Cargo.lock:5455: "thiserror 2.0.18",
Cargo.lock:5462:version = "0.33.1"
Cargo.lock:5471: "windows 0.57.0",
Cargo.lock:5476:version = "0.7.0"
Cargo.lock:5481: "core-foundation 0.9.4",
Cargo.lock:5487:version = "0.6.0"
Cargo.lock:5502: "getrandom 0.4.2",
Cargo.lock:5505: "windows-sys 0.61.2",
Cargo.lock:5510:version = "0.4.3"
Cargo.lock:5530:version = "0.34.1"
Cargo.lock:5540: "thiserror 2.0.18",
Cargo.lock:5541: "unicode-width 0.1.14",
Cargo.lock:5555:version = "2.0.18"
Cargo.lock:5559: "thiserror-impl 2.0.18",
Cargo.lock:5570: "syn 2.0.117",
Cargo.lock:5575:version = "2.0.18"
Cargo.lock:5581: "syn 2.0.117",
Cargo.lock:5595:version = "0.11.3"
Cargo.lock:5609:version = "0.3.47"
Cargo.lock:5624:version = "0.1.8"
Cargo.lock:5630:version = "0.2.27"
Cargo.lock:5640:version = "0.11.4"
Cargo.lock:5654:version = "0.11.4"
Cargo.lock:5665:version = "0.8.3"
Cargo.lock:5687: "windows-sys 0.61.2",
Cargo.lock:5698: "syn 2.0.117",
Cargo.lock:5703:version = "0.3.1"
Cargo.lock:5713:version = "0.26.4"
Cargo.lock:5723:version = "0.27.0"
Cargo.lock:5737:version = "0.7.18"
Cargo.lock:5750:version = "0.8.23"
Cargo.lock:5756: "toml_datetime 0.6.11",
Cargo.lock:5757: "toml_edit 0.22.27",
Cargo.lock:5762:version = "0.6.11"
Cargo.lock:5780:version = "0.22.27"
Cargo.lock:5787: "toml_datetime 0.6.11",
Cargo.lock:5789: "winnow 0.7.15",
Cargo.lock:5794:version = "0.25.11+spec-1.1.0"
Cargo.lock:5801: "winnow 1.0.3",
Cargo.lock:5810: "winnow 1.0.3",
Cargo.lock:5815:version = "0.1.2"
Cargo.lock:5821:version = "0.5.3"
Cargo.lock:5836:version = "0.6.11"
Cargo.lock:5854:version = "0.3.3"
Cargo.lock:5860:version = "0.3.3"
Cargo.lock:5866:version = "0.1.44"
Cargo.lock:5878:version = "0.1.31"
Cargo.lock:5884: "syn 2.0.117",
Cargo.lock:5889:version = "0.1.36"
Cargo.lock:5898:version = "0.26.9"
Cargo.lock:5912:version = "0.25.1"
Cargo.lock:5922:version = "0.24.2"
Cargo.lock:5932:version = "0.23.5"
Cargo.lock:5942:version = "0.23.4"
Cargo.lock:5952:version = "0.2.0"
Cargo.lock:5962:version = "0.3.5"
Cargo.lock:5972:version = "0.25.0"
Cargo.lock:5982:version = "0.23.5"
Cargo.lock:5992:version = "0.25.0"
Cargo.lock:6012:version = "0.1.7"
Cargo.lock:6018:version = "0.5.0"
Cargo.lock:6028:version = "0.25.0"
Cargo.lock:6059:version = "0.24.2"
Cargo.lock:6069:version = "0.25.0"
Cargo.lock:6079:version = "0.23.1"
Cargo.lock:6089:version = "0.24.2"
Cargo.lock:6099:version = "0.26.0"
Cargo.lock:6109:version = "0.7.2"
Cargo.lock:6119:version = "0.23.2"
Cargo.lock:6139:version = "0.2.5"
Cargo.lock:6145:version = "0.25.1"
Cargo.lock:6151:version = "0.27.0"
Cargo.lock:6161: "rand 0.9.4",
Cargo.lock:6163: "thiserror 2.0.18",
Cargo.lock:6169:version = "0.5.1"
Cargo.lock:6190: "windows-sys 0.61.2",
Cargo.lock:6201:version = "1.0.24"
Cargo.lock:6219: "unicode-width 0.1.14",
Cargo.lock:6224:version = "0.1.14"
Cargo.lock:6230:version = "0.2.0"
Cargo.lock:6236:version = "0.2.6"
Cargo.lock:6242:version = "0.2.11"
Cargo.lock:6248:version = "0.9.0"
Cargo.lock:6266:version = "0.7.6"
Cargo.lock:6278:version = "0.2.2"
Cargo.lock:6288: "getrandom 0.4.2",
Cargo.lock:6295:version = "0.8.0"
Cargo.lock:6305:version = "0.2.15"
Cargo.lock:6311:version = "0.9.5"
Cargo.lock:6317:version = "0.8.0"
Cargo.lock:6333:version = "0.3.1"
Cargo.lock:6342:version = "0.11.1+wasi-snapshot-preview1"
Cargo.lock:6348:version = "1.0.3+wasi-0.2.9"
Cargo.lock:6352: "wit-bindgen 0.57.1",
Cargo.lock:6357:version = "0.4.0+wasi-0.3.0-rc-2026-01-06"
Cargo.lock:6361: "wit-bindgen 0.51.0",
Cargo.lock:6366:version = "0.2.122"
Cargo.lock:6379:version = "0.4.72"
Cargo.lock:6389:version = "0.2.122"
Cargo.lock:6399:version = "0.2.122"
Cargo.lock:6406: "syn 2.0.117",
Cargo.lock:6412:version = "0.2.122"
Cargo.lock:6421:version = "0.244.0"
Cargo.lock:6431:version = "0.244.0"
Cargo.lock:6443:version = "0.4.2"
Cargo.lock:6456:version = "0.244.0"
Cargo.lock:6461: "hashbrown 0.15.5",
Cargo.lock:6468:version = "0.3.15"
Cargo.lock:6482:version = "0.31.14"
Cargo.lock:6494:version = "0.3.0"
Cargo.lock:6505:version = "0.31.14"
Cargo.lock:6516:version = "0.32.12"
Cargo.lock:6528:version = "20250721.0.1"
Cargo.lock:6541:version = "0.3.12"
Cargo.lock:6554:version = "0.3.12"
Cargo.lock:6567:version = "0.3.12"
Cargo.lock:6580:version = "0.31.10"
Cargo.lock:6585: "quick-xml 0.39.4",
Cargo.lock:6591:version = "0.31.11"
Cargo.lock:6603:version = "0.3.99"
Cargo.lock:6627: "core-foundation 0.10.1",
Cargo.lock:6631: "objc2 0.6.4",
Cargo.lock:6632: "objc2-foundation 0.3.2",
Cargo.lock:6648:version = "0.1.12"
Cargo.lock:6697: "thiserror 2.0.18",
Cargo.lock:6726: "ndk-sys 0.5.0+25.2.9519653",
Cargo.lock:6736: "thiserror 2.0.18",
Cargo.lock:6740: "windows 0.58.0",
Cargo.lock:6757:version = "7.0.3"
Cargo.lock:6769:version = "0.3.9"
Cargo.lock:6779:version = "0.4.0"
Cargo.lock:6785:version = "0.1.11"
Cargo.lock:6789: "windows-sys 0.61.2",
Cargo.lock:6794:version = "0.4.0"
Cargo.lock:6800:version = "0.57.0"
Cargo.lock:6804: "windows-core 0.57.0",
Cargo.lock:6805: "windows-targets 0.52.6",
Cargo.lock:6810:version = "0.58.0"
Cargo.lock:6814: "windows-core 0.58.0",
Cargo.lock:6815: "windows-targets 0.52.6",
Cargo.lock:6820:version = "0.57.0"
Cargo.lock:6824: "windows-implement 0.57.0",
Cargo.lock:6825: "windows-interface 0.57.0",
Cargo.lock:6826: "windows-result 0.1.2",
Cargo.lock:6827: "windows-targets 0.52.6",
Cargo.lock:6832:version = "0.58.0"
Cargo.lock:6836: "windows-implement 0.58.0",
Cargo.lock:6837: "windows-interface 0.58.0",
Cargo.lock:6838: "windows-result 0.2.0",
Cargo.lock:6839: "windows-strings 0.1.0",
Cargo.lock:6840: "windows-targets 0.52.6",
Cargo.lock:6845:version = "0.62.2"
Cargo.lock:6849: "windows-implement 0.60.2",
Cargo.lock:6850: "windows-interface 0.59.3",
Cargo.lock:6852: "windows-result 0.4.1",
Cargo.lock:6853: "windows-strings 0.5.1",
Cargo.lock:6858:version = "0.57.0"
Cargo.lock:6864: "syn 2.0.117",
Cargo.lock:6869:version = "0.58.0"
Cargo.lock:6875: "syn 2.0.117",
Cargo.lock:6880:version = "0.60.2"
Cargo.lock:6886: "syn 2.0.117",
Cargo.lock:6891:version = "0.57.0"
Cargo.lock:6897: "syn 2.0.117",
Cargo.lock:6902:version = "0.58.0"
Cargo.lock:6908: "syn 2.0.117",
Cargo.lock:6913:version = "0.59.3"
Cargo.lock:6919: "syn 2.0.117",
Cargo.lock:6924:version = "0.2.1"
Cargo.lock:6930:version = "0.6.1"
Cargo.lock:6935: "windows-result 0.4.1",
Cargo.lock:6936: "windows-strings 0.5.1",
Cargo.lock:6941:version = "0.1.2"
Cargo.lock:6945: "windows-targets 0.52.6",
Cargo.lock:6950:version = "0.2.0"
Cargo.lock:6954: "windows-targets 0.52.6",
Cargo.lock:6959:version = "0.4.1"
Cargo.lock:6968:version = "0.1.0"
Cargo.lock:6972: "windows-result 0.2.0",
Cargo.lock:6973: "windows-targets 0.52.6",
Cargo.lock:6978:version = "0.5.1"
Cargo.lock:6987:version = "0.52.0"
Cargo.lock:6991: "windows-targets 0.52.6",
Cargo.lock:6996:version = "0.59.0"
Cargo.lock:7000: "windows-targets 0.52.6",
Cargo.lock:7005:version = "0.60.2"
Cargo.lock:7009: "windows-targets 0.53.5",
Cargo.lock:7014:version = "0.61.2"
Cargo.lock:7023:version = "0.52.6"
Cargo.lock:7027: "windows_aarch64_gnullvm 0.52.6",
Cargo.lock:7028: "windows_aarch64_msvc 0.52.6",
Cargo.lock:7029: "windows_i686_gnu 0.52.6",
Cargo.lock:7030: "windows_i686_gnullvm 0.52.6",
Cargo.lock:7031: "windows_i686_msvc 0.52.6",
Cargo.lock:7032: "windows_x86_64_gnu 0.52.6",
Cargo.lock:7033: "windows_x86_64_gnullvm 0.52.6",
Cargo.lock:7034: "windows_x86_64_msvc 0.52.6",
Cargo.lock:7039:version = "0.53.5"
Cargo.lock:7044: "windows_aarch64_gnullvm 0.53.1",
Cargo.lock:7045: "windows_aarch64_msvc 0.53.1",
Cargo.lock:7046: "windows_i686_gnu 0.53.1",
Cargo.lock:7047: "windows_i686_gnullvm 0.53.1",
Cargo.lock:7048: "windows_i686_msvc 0.53.1",
Cargo.lock:7049: "windows_x86_64_gnu 0.53.1",
Cargo.lock:7050: "windows_x86_64_gnullvm 0.53.1",
Cargo.lock:7051: "windows_x86_64_msvc 0.53.1",
Cargo.lock:7056:version = "0.52.6"
Cargo.lock:7062:version = "0.53.1"
Cargo.lock:7068:version = "0.52.6"
Cargo.lock:7074:version = "0.53.1"
Cargo.lock:7080:version = "0.52.6"
Cargo.lock:7086:version = "0.53.1"
Cargo.lock:7092:version = "0.52.6"
Cargo.lock:7098:version = "0.53.1"
Cargo.lock:7104:version = "0.52.6"
Cargo.lock:7110:version = "0.53.1"
Cargo.lock:7116:version = "0.52.6"
Cargo.lock:7122:version = "0.53.1"
Cargo.lock:7128:version = "0.52.6"
Cargo.lock:7134:version = "0.53.1"
Cargo.lock:7140:version = "0.52.6"
Cargo.lock:7146:version = "0.53.1"
Cargo.lock:7152:version = "0.30.13"
Cargo.lock:7162: "calloop 0.13.0",
Cargo.lock:7165: "core-foundation 0.9.4",
Cargo.lock:7173: "objc2 0.5.2",
Cargo.lock:7174: "objc2-app-kit 0.2.2",
Cargo.lock:7175: "objc2-foundation 0.2.2",
Cargo.lock:7181: "redox_syscall 0.4.1",
Cargo.lock:7182: "rustix 0.38.44",
Cargo.lock:7184: "smithay-client-toolkit 0.19.2",
Cargo.lock:7196: "windows-sys 0.52.0",
Cargo.lock:7204:version = "0.7.15"
Cargo.lock:7213:version = "1.0.3"
Cargo.lock:7222:version = "0.0.19"
Cargo.lock:7228:version = "0.51.0"
Cargo.lock:7237:version = "0.57.1"
Cargo.lock:7243:version = "0.51.0"
Cargo.lock:7254:version = "0.51.0"
Cargo.lock:7262: "syn 2.0.117",
Cargo.lock:7270:version = "0.51.0"
Cargo.lock:7278: "syn 2.0.117",
Cargo.lock:7285:version = "0.244.0"
Cargo.lock:7304:version = "0.244.0"
Cargo.lock:7322:version = "0.6.3"
Cargo.lock:7339:version = "0.13.2"
Cargo.lock:7354:version = "0.13.2"
Cargo.lock:7360:version = "0.3.10"
Cargo.lock:7371: "windows-sys 0.59.0",
Cargo.lock:7376:version = "0.4.2"
Cargo.lock:7389:version = "0.2.1"
Cargo.lock:7395:version = "0.8.28"
Cargo.lock:7401:version = "0.4.5"
Cargo.lock:7410:version = "0.8.2"
Cargo.lock:7421:version = "0.8.2"
Cargo.lock:7427: "syn 2.0.117",
Cargo.lock:7455: "rand 0.8.6",
Cargo.lock:7462: "windows-sys 0.52.0",
Cargo.lock:7471:version = "0.4.4"
Cargo.lock:7481:version = "0.4.4"
Cargo.lock:7487: "syn 2.0.117",
Cargo.lock:7502: "syn 2.0.117",
Cargo.lock:7523: "quick-xml 0.30.0",
Cargo.lock:7532:version = "0.8.48"
Cargo.lock:7541:version = "0.8.48"
Cargo.lock:7547: "syn 2.0.117",
Cargo.lock:7552:version = "0.1.8"
Cargo.lock:7561:version = "0.1.7"
Cargo.lock:7567: "syn 2.0.117",
Cargo.lock:7588: "syn 2.0.117",
Cargo.lock:7593:version = "0.2.4"
Cargo.lock:7604:version = "0.11.6"
Cargo.lock:7615:version = "0.11.3"
Cargo.lock:7621: "syn 2.0.117",
Cargo.lock:7626:version = "1.0.21"
Cargo.lock:7632:version = "0.5.1"
Cargo.lock:7638:version = "0.5.15"
Cargo.lock:7667: "syn 2.0.117",
Cargo.lock:7679: "syn 2.0.117",
crates/imp-llm/src/model.rs:345:                cache_read_per_mtok: 0.3,
crates/imp-llm/src/model.rs:364:                cache_read_per_mtok: 0.1,
crates/imp-llm/src/model.rs:402:                cache_read_per_mtok: 0.125,
crates/imp-llm/src/model.rs:418:                input_per_mtok: 0.30,
crates/imp-llm/src/model.rs:421:                cache_write_per_mtok: 0.30,
crates/imp-llm/src/model.rs:437:                input_per_mtok: 0.27,
crates/imp-llm/src/model.rs:440:                cache_write_per_mtok: 0.27,
crates/imp-llm/src/model.rs:457:                cache_read_per_mtok: 0.14,
crates/imp-llm/src/model.rs:678:                cache_read_per_mtok: 0.25,
crates/imp-llm/src/model.rs:712:                input_per_mtok: 0.20,
crates/imp-llm/src/model.rs:715:                cache_write_per_mtok: 0.20,
crates/imp-llm/src/model.rs:732:                cache_read_per_mtok: 0.175,
crates/imp-llm/src/model.rs:750:                cache_read_per_mtok: 0.175,
crates/imp-llm/src/model.rs:962:                cache_read_per_mtok: 0.275,
crates/imp-llm/src/usage.rs:102:            cache_read_per_mtok: 0.3,
crates/imp-llm/src/usage.rs:111:        // 200k cache_read * $0.30/Mtok = $0.06
crates/imp-llm/src/usage.rs:113:        // 100k cache_write * $3.75/Mtok = $0.375
crates/imp-llm/src/usage.rs:114:        assert!((cost.cache_write - 0.375).abs() < f64::EPSILON);
crates/imp-llm/src/usage.rs:115:        // total = 3.0 + 7.5 + 0.06 + 0.375 = 10.935
crates/imp-llm/src/usage.rs:125:            cache_read_per_mtok: 0.3,
crates/imp-llm/src/usage.rs:164:            cache_write: 0.25,
crates/imp-llm/src/usage.rs:170:            cache_read: 0.25,
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:273:  "cost": 0.12,
docs/proposals/mana-wiki-schema-and-workflow.md:3:> Proposal for `.10.2` — April 2026
docs/proposals/mana-wiki-schema-and-workflow.md:9:> Depends on: `.10.1` (imp memory architecture and mana ownership boundaries)
docs/proposals/mana-wiki-schema-and-workflow.md:179:- "Project uses serde_yml 0.0.12" → `mana fact` with `grep` verify.
docs/proposals/mana-wiki-schema-and-workflow.md:386:The project uses serde_yml 0.0.12 ([fact 112: "serde_yml version"]).
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:94:│ ✓ .10.3 closed: Strengthen mana-first prompt │
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:217:| ".10.3 closed" | Work state | Notification banner | Until next user input |
docs/proposals/mana-aware-runtime-context-read-path.md:9:> Depends on: `.10.1` (memory architecture), `.10.2` (wiki schema)
crates/imp-llm/src/providers/anthropic.rs:1165:                cache_read_per_mtok: 0.3,
crates/imp-llm/src/providers/anthropic.rs:1183:                cache_read_per_mtok: 0.3,
docs/proposals/imp-memory-architecture-and-mana-boundary.md:3:> Proposal for `.10.1` — April 2026
docs/proposals/imp-memory-architecture-and-mana-boundary.md:185:workflow are defined in `.10.2`.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:282:- Strengthen prompt doctrine to distinguish the 4 layers (`.10.3`).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:285:- Define wiki schema and maintenance operations (`.10.2`).
crates/imp-llm/src/providers/openai_codex.rs:198:            "temperature": 0.2,
crates/imp-llm/src/providers/google.rs:737:                cache_read_per_mtok: 0.315,
crates/imp-llm/src/providers/google.rs:753:                input_per_mtok: 0.15,
crates/imp-llm/src/providers/google.rs:756:                cache_write_per_mtok: 0.15,
crates/imp-llm/src/providers/google.rs:908:            temperature: Some(0.2),
crates/imp-llm/src/providers/google.rs:948:                - 0.2)
docs/release-promotions/commit-board.html:59:<script id="commit-data" type="application/json">[{&quot;sha&quot;: &quot;4e7f7464e1ef12b17dee43636fdfdebf8385ad59&quot;, &quot;short&quot;: &quot;4e7f746&quot;, &quot;subject&quot;: &quot;Reduce imp TUI startup latency&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-19T11:38:14-07:00&quot;, &quot;side&quot;: &quot;nightly-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-llm/src/auth.rs&quot;, &quot;crates/imp-llm/src/providers/openai.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 214, &quot;deletions&quot;: 43, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;moderate churn (257 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-llm/src/providers&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;d6521026f113f6fe80b5f55150cf66658190289f&quot;, &quot;short&quot;: &quot;d652102&quot;, &quot;subject&quot;: &quot;Prepare vanilla imp release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T14:02:13-07:00&quot;, &quot;side&quot;: &quot;nightly-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;.mana/.3-set-up-harbor-adapter-and-terminal-bench-20-runner.md&quot;, &quot;.mana/.5-add-safe-automatic-context-compaction-for-long-run.md&quot;, &quot;.mana/.5.1-add-disabled-by-default-auto-compaction-config-sca.md&quot;, &quot;.mana/.6-hardening-pass-reduce-bugs-and-contract-mismatches.md&quot;, &quot;.mana/.6.6-enforce-lua-extension-capability-boundaries.md&quot;, &quot;.mana/.6.7-propagate-cancellation-into-active-tool-execution.md&quot;, &quot;.mana/.6.8-align-diff-tool-registration-with-mode-contracts.md&quot;, &quot;.mana/.9-upgrade-imp-mana-authoring-prompt-contract-for-orc.md&quot;, &quot;.mana/.gitignore&quot;, &quot;.mana/21-imp-efficiency-smarter-tool-output-truncation.md&quot;, &quot;.mana/245.1-define-manaimp-contract-implications-of-file-nativ.md&quot;, &quot;.mana/245.1.1-define-vnext-manaimp-subagent-handoff-packet-for-o.md&quot;, &quot;.mana/248-comprehensive-imp-uiux-review-upgrade-and-polish-a.md&quot;, &quot;.mana/248.14-implement-restrained-ansi-emphasis-for-shell-typog.md&quot;, &quot;.mana/248.16.5-create-svg-wireframes-for-candidate-imp-tui-layout.md&quot;, &quot;.mana/248.16.7-revise-imp-tui-wireframes-and-core-memo-after-user.md&quot;, &quot;.mana/248.17-design-terminal-emulator-native-coding-agent-cockp.md&quot;, &quot;.mana/248.18-harden-and-humanize-imp-error-streaming-across-pro.md&quot;, &quot;.mana/248.18.1-extract-shared-imp-core-streamed-error-normalizati.md&quot;, &quot;.mana/248.18.2-harden-imp-core-partial-stream-and-silent-eof-diag.md&quot;, &quot;.mana/248.18.3-design-stable-machine-facing-streamed-error-envelo.md&quot;, &quot;.mana/248.7-plan-shared-uxruntime-seams-for-shell-tui-and-view.md&quot;, &quot;.mana/248.9-capture-and-sequence-real-user-feedback-on-the-new.md&quot;, &quot;.mana/249-reduce-duplicate-verbose-mana-change-narration-in.md&quot;, &quot;.mana/250-shape-getimpdev-landing-page-direction-and-impleme.md&quot;, &quot;.mana/254-fresh-smoke-trial-for-native-imp-run-on-an-isolate.md&quot;, &quot;.mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md&quot;, &quot;.mana/257-draft-imp-ontologymd-for-shared-featureruntime-lan.md&quot;, &quot;.mana/259-audit-panic-usage-and-detached-task-failure-policy.md&quot;, &quot;.mana/263-audit-and-isolate-library-level-stderr-writes-that.md&quot;, &quot;.mana/263.2-classify-mana-core-stderr-writes-for-embedded-risk.md&quot;, &quot;.mana/264-normalize-imp-storage-topology-for-sessions-config.md&quot;, &quot;.mana/264.1-audit-current-imp-durable-storage-surfaces-and-pat.md&quot;, &quot;.mana/264.2-specify-normalized-imp-storage-contract-and-migrat.md&quot;, &quot;.mana/264.3.1-add-shared-imp-core-storage-path-module-for-canoni.md&quot;, &quot;.mana/264.3.2-migrate-config-auth-session-and-session-search-cal.md&quot;, &quot;.mana/264.3.3-migrate-instruction-discovery-to-canonical-impagen.md&quot;, &quot;.mana/264.3.4-implement-non-destructive-migration-from-legacy-im.md&quot;, &quot;.mana/264.4-audit-and-fix-imp-session-index-lifecycle-wiring-f.md&quot;, &quot;.mana/264.6-decide-canonical-imp-filesystem-roots-for-global-a.md&quot;, &quot;.mana/264.7-specify-precedence-and-merge-rules-between-imp-and.md&quot;, &quot;.mana/264.8-specify-migration-from-xdgmacos-legacy-paths-into.md&quot;, &quot;.mana/266-cross-codebase-review-compare-imp-and-hermes-agent.md&quot;, &quot;.mana/266.1-design-adoption-path-provider-resilience-and-auth.md&quot;, &quot;.mana/266.2-design-adoption-path-session-recall-memory-and-con.md&quot;, &quot;.mana/266.3-design-adoption-path-extension-seams-and-product-s.md&quot;, &quot;.mana/266.4-imp-vnext-ranked-roadmap-and-phased-execution-plan.md&quot;, &quot;.mana/266.4.7-phase-5-epic-selective-later-product-surface-expan.md&quot;, &quot;.mana/267-adopt-highest-value-product-lessons-from-opencode.md&quot;, &quot;.mana/268.1-diagnose-native-imp-mana-tool-divergence-from-cli.md&quot;, &quot;.mana/27-improve-mana-pool-competitive-grade-dispatch-engin.md&quot;, &quot;.mana/27.14-define-attempt-scoped-autonomy-observation-record.md&quot;, &quot;.mana/27.2-imp-ui-compact-mana-statusprogress-surface.md&quot;, &quot;.mana/271-add-native-youtube-video-interpretation-support-to.md&quot;, &quot;.mana/271.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/271.2-harden-imp-spawn-and-mana-closetool-execution-agai.md&quot;, &quot;.mana/272-add-native-video-context-ingestion-architecture-fo.md&quot;, &quot;.mana/272.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/272.2-design-richer-video-interpretation-beyond-transcri.md&quot;, &quot;.mana/273-diagnose-and-harden-kimi-code-oauth-model-routing.md&quot;, &quot;.mana/273.5-sprint-import-and-execute-pi-typescript-extensions.md&quot;, &quot;.mana/273.5.10-prove-bun-ts-adapter-against-local-pi-color-palett.md&quot;, &quot;.mana/273.5.11-add-official-pi-dynamic-tools-compatibility-fixtur.md&quot;, &quot;.mana/273.5.12-define-sprint-1-typescriptpi-extension-support-bou.md&quot;, &quot;.mana/273.5.13-probe-dependency-bearing-pi-extension-compatibilit.md&quot;, &quot;.mana/273.5.4-normalize-typeboxjson-schemas-from-typescript-exte.md&quot;, &quot;.mana/275-assess-and-sequence-next-llm-oauth-provider-integr.md&quot;, &quot;.mana/275.10-inventory-pi-and-imp-provideroauth-surfaces.md&quot;, &quot;.mana/275.11-sequence-pi-provideroauth-parity-implementation.md&quot;, &quot;.mana/275.6-assess-pi-google-antigravity-provider-route-for-im.md&quot;, &quot;.mana/275.9-research-unofficial-cursor-provider-support-for-im.md&quot;, &quot;.mana/276-investigate-and-harden-tui-esc-cancellation-for-hu.md&quot;, &quot;.mana/277-fix-imp-tui-clean-ui-corruption-and-string-join-ov.md&quot;, &quot;.mana/278-fix-inspector-mode-interaction-model.md&quot;, &quot;.mana/28.1-make-imp-run-the-canonical-mana-worker-runtime-whi.md&quot;, &quot;.mana/28.5.1-patch-imp-system-prompt-with-mana-first-planning-d.md&quot;, &quot;.mana/28.5.6-implement-turn-scoped-mana-review-packet-aggregati.md&quot;, &quot;.mana/28.5.7-render-between-turn-mana-review-packets-across-imp.md&quot;, &quot;.mana/28.5.7.1-add-shared-imp-core-turnmanadelta-renderer-and-man.md&quot;, &quot;.mana/28.5.7.2-render-compact-between-turn-mana-block-and-textual.md&quot;, &quot;.mana/28.5.7.3-render-between-turn-mana-review-packets-in-imp-cli.md&quot;, &quot;.mana/28.5.7.4-add-shared-manareviewmode-config-and-presentation.md&quot;, &quot;.mana/28.5.7.5-wire-imp-tui-compact-widget-tray-block-and-sidebar.md&quot;, &quot;.mana/280-review-project-gaps-that-would-make-imp-stronger-t.md&quot;, &quot;.mana/280.1-run-dirac-evals-with-imp-using-gemini-secret.md&quot;, &quot;.mana/280.2-adopt-dirac-inspired-code-intelligence-and-precise.md&quot;, &quot;.mana/280.2.1.1-decide-migration-safe-naming-strategy-for-imp-scan.md&quot;, &quot;.mana/280.2.2-implement-read-oriented-symbol-extraction-and-skel.md&quot;, &quot;.mana/280.2.3-add-anchor-backed-read-and-stale-safe-edit-flow-to.md&quot;, &quot;.mana/280.2.4-implement-edit-transaction-batching-with-combined.md&quot;, &quot;.mana/282-design-native-scoped-secret-injection-for-command.md&quot;, &quot;.mana/285-verify-installed-imp-binary-includes-latest-secret.md&quot;, &quot;.mana/290-complete-imp-codebase-quality-audit.md&quot;, &quot;.mana/290.1-split-imp-tui-apprs-by-responsibility.md&quot;, &quot;.mana/290.2-split-imp-core-agentrs-into-focused-runtime-module.md&quot;, &quot;.mana/290.3-split-imp-cli-librs-into-command-modules.md&quot;, &quot;.mana/290.4-split-native-mana-tool-implementation-into-focused.md&quot;, &quot;.mana/291-rewrite-imp-readme-around-product-features-mana-an.md&quot;, &quot;.mana/31.2-add-guardrail-config-types-and-profile-selection-t.md&quot;, &quot;.mana/31.3-integrate-guardrails-into-the-imp-system-prompt-an.md&quot;, &quot;.mana/31.4-add-the-initial-zig-guardrail-profile-and-document.md&quot;, &quot;.mana/33-chat-view-replace-duplicated-animation-logic-with.md&quot;, &quot;.mana/34-sidebar-detail-header-use-spinnerframe-and-respect.md&quot;, &quot;.mana/35-editor-remove-dead-tick-and-animationlevel-params.md&quot;, &quot;.mana/36-animation-config-reconcile-minimal-namingdocs-afte.md&quot;, &quot;.mana/37-add-first-class-usage-accounting-and-reporting-to.md&quot;, &quot;.mana/37.5-test-and-document-imp-usage-accountingreporting.md&quot;, &quot;.mana/41-anthropic-api-parity-adopt-claude-code-patterns-fo.md&quot;, &quot;.mana/44-define-memory-and-code-intelligence-architecture-f.md&quot;, &quot;.mana/44.1-author-guest-runtime-extension-substrate-proposal.md&quot;, &quot;.mana/44.1.10-implement-documentworkspace-symbols-with-ast-first.md&quot;, &quot;.mana/44.1.11-implement-hover-and-signature-help-on-the-phase-1.md&quot;, &quot;.mana/44.1.12-unify-code-intelligence-diagnostic-summaries-with.md&quot;, &quot;.mana/44.1.14-evaluate-whether-repeated-evidence-promotion-flows.md&quot;, &quot;.mana/44.1.5-plan-guarded-write-oriented-semantic-actions-and-p.md&quot;, &quot;.mana/44.1.5.5-specify-semantic-write-execution-contract-for-prev.md&quot;, &quot;.mana/44.1.6-sequence-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.6.1-define-shared-normalization-envelopes-for-read-ori.md&quot;, &quot;.mana/44.1.6.2-plan-diagnostics-plus-ast-alignment-for-the-first.md&quot;, &quot;.mana/44.1.6.3-plan-document-symbols-and-go-to-definition-over-th.md&quot;, &quot;.mana/44.1.6.4-plan-references-and-workspace-symbol-browsing-for.md&quot;, &quot;.mana/44.1.6.5-plan-hover-and-signature-enrichment-after-core-rea.md&quot;, &quot;.mana/44.1.7-roll-out-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.8-normalize-read-oriented-code-intelligence-queryres.md&quot;, &quot;.mana/44.1.9-implement-phase-1-diagnostics-go-to-definition-and.md&quot;, &quot;.mana/44.3-translate-guest-runtime-design-into-phased-impleme.md&quot;, &quot;.mana/45-tower-rebuild-around-explicit-contracts-durable-le.md&quot;, &quot;.mana/45.10.5-update-docs-for-mana-platform-substrate-and-imp-pr.md&quot;, &quot;.mana/45.11-capture-near-term-imp-execution-lanes-under-the-im.md&quot;, &quot;.mana/45.11.1-resolve-consequential-defaults-for-near-term-imp-i.md&quot;, &quot;.mana/45.11.1.1-clarify-whether-native-rust-not-lua-applies-to-imp.md&quot;, &quot;.mana/45.11.1.2-sequence-near-term-imp-implementation-order-from-s.md&quot;, &quot;.mana/45.4-phase-3-introduce-runner-protocol-and-local-adapte.md&quot;, &quot;.mana/45.4.2-plan-the-first-imp-local-runner-adapter-that-consu.md&quot;, &quot;.mana/45.4.4-plan-the-cutover-from-current-imp-run-plus-mana-ru.md&quot;, &quot;.mana/45.5-phase-4-rebuild-imp-around-stable-workerruntime-se.md&quot;, &quot;.mana/45.5.1-map-imp-core-hotspots-into-target-runtime-context.md&quot;, &quot;.mana/45.5.3-write-a-compact-imp-decomposition-order-for-post-c.md&quot;, &quot;.mana/45.7-phase-6-harden-policy-isolation-and-migration-surf.md&quot;, &quot;.mana/45.7.4-evaluate-whether-imp-should-add-a-native-gitrepo-t.md&quot;, &quot;.mana/46-broaden-imp-attention-beyond-toolsprompting-under.md&quot;, &quot;.mana/46.1-reconcile-long-session-runtime-safety-gaps-and-tur.md&quot;, &quot;.mana/46.2-reconcile-user-visible-discoverability-and-ux-brea.md&quot;, &quot;.mana/46.2.1-surface-usage-reporting-in-the-tui-commandhelpstar.md&quot;, &quot;.mana/47-rebuild-imp-around-explicit-runtime-boundaries-pro.md&quot;, &quot;.mana/47.1-contracts-and-ownership-boundary-for-mana-imp-rebu.md&quot;, &quot;.mana/47.6-sequence-the-imp-rebuild-as-an-incremental-migrati.md&quot;, &quot;.mana/50-define-cli-first-operator-surface-for-imp-with-tui.md&quot;, &quot;.mana/50.10-implement-guided-cli-parity-flows-for-settings-per.md&quot;, &quot;.mana/50.10.1-implement-terminal-native-imp-settings-flow-for-cl.md&quot;, &quot;.mana/50.10.1.2-let-imp-chat-no-tools-reach-the-shell-without-prov.md&quot;, &quot;.mana/50.10.2-implement-terminal-native-imp-personality-flow-for.md&quot;, &quot;.mana/50.11-implement-first-shell-to-view-handoff-for-sessions.md&quot;, &quot;.mana/50.11.2-align-imp-chat-view-handoff-with-explicit-imp-view.md&quot;, &quot;.mana/50.12-flip-plain-imp-to-imp-chat-after-shell-readiness-g.md&quot;, &quot;.mana/50.13-plan-extraction-of-shared-fullscreen-consumed-runt.md&quot;, &quot;.mana/50.14-specify-the-shared-imp-ui-request-and-runtime-even.md&quot;, &quot;.mana/50.16-follow-on-cli-native-affordance-stack-after-505-se.md&quot;, &quot;.mana/50.16.1-define-stable-human-vs-machine-output-modes-across.md&quot;, &quot;.mana/50.16.2-plan-cli-first-checkpoint-productization-after-out.md&quot;, &quot;.mana/50.16.3-plan-visible-cli-first-planning-artifacts-and-exec.md&quot;, &quot;.mana/50.16.4-plan-first-class-approval-policy-layer-for-cli-fir.md&quot;, &quot;.mana/50.16.5-surface-session-browsing-and-session-search-as-fir.md&quot;, &quot;.mana/50.16.5.1-audit-and-reconcile-imp-session-storage-and-sessio.md&quot;, &quot;.mana/50.16.6-plan-detachedbackground-local-execution-after-cli.md&quot;, &quot;.mana/50.17-define-stable-human-vs-machine-output-contracts-fo.md&quot;, &quot;.mana/50.18-define-cli-first-session-browsing-and-sessionsearc.md&quot;, &quot;.mana/50.19-define-stable-imp-human-vs-machine-output-contract.md&quot;, &quot;.mana/50.20-plan-first-cli-first-checkpoint-productization-ove.md&quot;, &quot;.mana/50.21-specify-visible-planning-artifacts-and-checklist-b.md&quot;, &quot;.mana/50.22-specify-the-first-visible-planning-workflow-and-pl.md&quot;, &quot;.mana/50.23-specify-cli-first-approval-policy-and-blocked-stat.md&quot;, &quot;.mana/50.24-define-the-first-cli-first-approval-policy-surface.md&quot;, &quot;.mana/50.25-specify-detachedbackground-local-execution-contrac.md&quot;, &quot;.mana/50.26-define-the-first-local-detachedbackground-executio.md&quot;, &quot;.mana/50.6-design-the-cli-first-interactive-shell-path-for-im.md&quot;, &quot;.mana/50.9-implement-the-first-cli-first-proving-slice-with-e.md&quot;, &quot;.mana/51.6.1-audit-current-mana-core-embedding-surface-against.md&quot;, &quot;.mana/65-root-mana-currently-lists-child-513-but-direct-sho.md&quot;, &quot;.mana/69-imp-cli-no-longer-contains-duplicate-mana-unit-loa.md&quot;, &quot;.mana/73-code-intelligence-outputs-are-transient-by-default.md&quot;, &quot;.mana/81-design-imp-native-delegation-tool-around-imp-run-a.md&quot;, &quot;.mana/81.10-define-codemap-backed-context-seam-for-imp-run-and.md&quot;, &quot;.mana/82-assess-gpt-54-pro-support-through-openai-chatgpt-o.md&quot;, &quot;.mana/82.2-add-gpt-54-pro-to-imp-model-registry-only-after-oa.md&quot;, &quot;.mana/83-harden-imp-tui-text-box-cursor-and-bounds-handling.md&quot;, &quot;.mana/RULES.md&quot;, &quot;.mana/archive/2026/03/.2-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/16-imp-core-hardening-production-ready-agent-engine.md&quot;, &quot;.mana/archive/2026/03/16.1-wire-config-agent-agentbuilder-thresholds-hooks-re.md&quot;, &quot;.mana/archive/2026/03/16.2-tool-argument-validation-json-schema-before-execut.md&quot;, &quot;.mana/archive/2026/03/16.3-llm-retry-with-exponential-backoff-and-jitter.md&quot;, &quot;.mana/archive/2026/03/16.4-loop-detection-prevent-infinite-tool-call-retry-lo.md&quot;, &quot;.mana/archive/2026/03/16.5-file-not-found-suggestions-with-levenshtein-rankin.md&quot;, &quot;.mana/archive/2026/03/16.6-auto-resume-after-compaction-re-queue-original-pro.md&quot;, &quot;.mana/archive/2026/03/16.7-file-read-tracking-and-staleness-detection.md&quot;, &quot;.mana/archive/2026/03/16.8-file-version-history-pre-edit-snapshots-for-rollba.md&quot;, &quot;.mana/archive/2026/03/17-imp-efficiency-enable-prompt-caching.md&quot;, &quot;.mana/archive/2026/03/19-imp-efficiency-in-session-file-content-cache.md&quot;, &quot;.mana/archive/2026/03/20-imp-efficiency-parallelize-grep-block-search-with.md&quot;, &quot;.mana/archive/2026/03/229-imp-rust-coding-agent-engine.md&quot;, &quot;.mana/archive/2026/03/229.1-workspace-setup-imp-llm-types.md&quot;, &quot;.mana/archive/2026/03/229.10-imp-llm-anthropic-oauth.md&quot;, &quot;.mana/archive/2026/03/229.11-imp-core-hook-system.md&quot;, &quot;.mana/archive/2026/03/229.12-imp-core-tree-sitter-tools-probesearch-probeextrac.md&quot;, &quot;.mana/archive/2026/03/229.13-imp-core-config-resource-discovery.md&quot;, &quot;.mana/archive/2026/03/229.14-imp-core-system-prompt-assembly.md&quot;, &quot;.mana/archive/2026/03/229.15-imp-lua-lua-extension-runtime.md&quot;, &quot;.mana/archive/2026/03/229.16-imp-core-shell-tool-loader.md&quot;, &quot;.mana/archive/2026/03/229.17-imp-tui-ratatui-interactive-mode.md&quot;, &quot;.mana/archive/2026/03/229.18-imp-cli-binary-entry-point.md&quot;, &quot;.mana/archive/2026/03/229.2-imp-llm-anthropic-provider.md&quot;, &quot;.mana/archive/2026/03/229.3-imp-core-tool-trait-file-tools-read-write-edit-mul.md&quot;, &quot;.mana/archive/2026/03/229.4-imp-core-bash-grep-find-tools.md&quot;, &quot;.mana/archive/2026/03/229.5-imp-core-ask-diff-tools.md&quot;, &quot;.mana/archive/2026/03/229.6-imp-core-agent-loop.md&quot;, &quot;.mana/archive/2026/03/229.7-imp-core-session-manager.md&quot;, &quot;.mana/archive/2026/03/229.8-imp-core-context-management-observation-masking-co.md&quot;, &quot;.mana/archive/2026/03/229.9-imp-llm-openai-google-providers.md&quot;, &quot;.mana/archive/2026/03/23-learning-loop-agent-curated-memory-skill-managemen.md&quot;, &quot;.mana/archive/2026/03/23.1-system-prompt-layer-6-wire-memory-into-prompt-asse.md&quot;, &quot;.mana/archive/2026/03/23.2-memory-store-and-memory-tool.md&quot;, &quot;.mana/archive/2026/03/23.3-skill-manage-tool-agent-creates-patches-and-delete.md&quot;, &quot;.mana/archive/2026/03/23.4-learning-nudges-system-prompt-text-and-onagentend.md&quot;, &quot;.mana/archive/2026/03/23.5-session-index-with-fts5-full-text-search.md&quot;, &quot;.mana/archive/2026/03/23.6-session-search-tool.md&quot;, &quot;.mana/archive/2026/03/24-tui-ux-overhaul-information-density-summaries-inte.md&quot;, &quot;.mana/archive/2026/03/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/03/24.2-progress-indicator-in-status-bar-during-streaming.md&quot;, &quot;.mana/archive/2026/03/24.3-per-tool-call-expandcollapse-and-auto-expand-error.md&quot;, &quot;.mana/archive/2026/03/24.4-turn-end-summary-with-file-change-tracking.md&quot;, &quot;.mana/archive/2026/03/24.5-visual-separation-of-tool-activity-from-assistant.md&quot;, &quot;.mana/archive/2026/03/24.6-editor-polish-placeholder-model-indicator-keybindi.md&quot;, &quot;.mana/archive/2026/03/24.7-fix-context-window-tracking-use-actual-conversatio.md&quot;, &quot;.mana/archive/2026/03/24.8-approval-flow-wire-userinterface-for-tool-confirma.md&quot;, &quot;.mana/archive/2026/03/25-multi-provider-llm-support-with-data-driven-welcom.md&quot;, &quot;.mana/archive/2026/03/25.1-provider-metadata-registry-auth-generalization.md&quot;, &quot;.mana/archive/2026/03/25.2-openai-compatible-chat-completions-provider.md&quot;, &quot;.mana/archive/2026/03/25.3-add-builtin-models-for-new-providers.md&quot;, &quot;.mana/archive/2026/03/25.4-data-driven-welcome-flow.md&quot;, &quot;.mana/archive/2026/03/25.5-generalize-cli-login-for-all-providers.md&quot;, &quot;.mana/archive/2026/03/26-fix-imp-tui-compile-errors-around-toolcallorder-re.md&quot;, &quot;.mana/archive/2026/03/27.1-imp-core-mana-tool-add-native-orchestration-action.md&quot;, &quot;.mana/archive/2026/03/31-add-configurable-engineering-guardrails-to-imp.md&quot;, &quot;.mana/archive/2026/03/37.1-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/37.2-persist-canonical-usage-entries-in-imp-core-sessio.md&quot;, &quot;.mana/archive/2026/03/37.3-unify-usage-persistence-across-imp-execution-paths.md&quot;, &quot;.mana/archive/2026/03/37.4-add-imp-usage-reporting-commands-and-export.md&quot;, &quot;.mana/archive/2026/04/.10-define-clean-mana-vs-imp-boundary-and-memory-conso.md&quot;, &quot;.mana/archive/2026/04/.10.1-define-imp-memory-layer-architecture-and-mana-ownership-boundaries.md&quot;, &quot;.mana/archive/2026/04/.10.2-design-a-mana-wiki-schema-and-knowledge-maintenance-workflow.md&quot;, &quot;.mana/archive/2026/04/.10.3-strengthen-mana-first-prompt-doctrine-for-durable-planning.md&quot;, &quot;.mana/archive/2026/04/.10.4-design-mana-aware-runtime-context-read-path-for-prompt-assembly.md&quot;, &quot;.mana/archive/2026/04/.10.5-design-inline-mana-state-and-knowledge-surfaces-for-imp-runtime.md&quot;, &quot;.mana/archive/2026/04/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/04/266.4.3.4-fix-stale-secret-metadata-and-missing-keychain-dia.md&quot;, &quot;.mana/archive/2026/04/27.4-imp-promptingtool-guidance-prefer-native-mana-tool.md&quot;, &quot;.mana/archive/2026/04/272-add-kimi-model-compatibility-and-fix-ctrll-model-p.md&quot;, &quot;.mana/archive/2026/04/274-audit-and-simplify-imp-core-config-module.md&quot;, &quot;.mana/archive/2026/04/28-surface-built-in-features-already-implemented-in-i.md&quot;, &quot;.mana/archive/2026/04/28.1.1-specify-the-strengthened-imp-run-worker-contract-a.md&quot;, &quot;.mana/archive/2026/04/28.1.2-implement-reusable-imp-side-mana-unit-worker-runti.md&quot;, &quot;.mana/archive/2026/04/28.1.3-integrate-mana-run-with-the-strengthened-imp-run-w.md&quot;, &quot;.mana/archive/2026/04/28.1.5-fix-native-imp-delegate-worker-defaults-for-openai.md&quot;, &quot;.mana/archive/2026/04/28.1.5-make-imps-native-mana-tool-the-clear-first-class-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.2-fix-direct-imp-run-codexopenai-worker-request-defa.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.2-extract-shared-model-first-runtime-connection-reso.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.3-refactor-headless-worker-auth-to-normalize-empty-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.4-clarify-imp-to-imp-tool-vocabulary-and-align-docs.md&quot;, &quot;.mana/archive/2026/04/29.3-add-recent-session-previews-to-the-imp-startup-pan.md&quot;, &quot;.mana/archive/2026/04/29.4-add-context-aware-quickstart-guidance-and-health-s.md&quot;, &quot;.mana/archive/2026/04/29.6.1-implement-native-mana-scope-targeting-in-imp-tool.md&quot;, &quot;.mana/archive/2026/04/29.6.2-implement-safe-partial-mana-update-semantics-in-im.md&quot;, &quot;.mana/archive/2026/04/29.6.3-implement-append-style-mana-actions-for-conversati.md&quot;, &quot;.mana/archive/2026/04/30-render-compact-widgetstatus-surfaces-already-suppo.md&quot;, &quot;.mana/archive/2026/04/31.1-write-the-engineering-guardrails-design-note-for-i.md&quot;, &quot;.mana/archive/2026/04/32-productize-checkpoints-from-imps-existing-file-sna.md&quot;, &quot;.mana/archive/2026/04/32.1-checkpoint-foundation-shared-filehistory-wiring-an.md&quot;, &quot;.mana/archive/2026/04/32.2-checkpoint-persistence-session-custom-records-plus.md&quot;, &quot;.mana/archive/2026/04/32.3-checkpoint-ux-minimal-slash-command-list-and-resto.md&quot;, &quot;.mana/archive/2026/04/42-per-agent-cached-context-assembly-for-mana-dispatc.md&quot;, &quot;.mana/archive/2026/04/47.1.4-implement-the-first-shared-verifier-and-evidence-r.md&quot;, &quot;.mana/index.yaml.old&quot;, &quot;.mana/migration-conflicts/.3-add-secure-generic-credential-storage-and-lua-secr.md.txt&quot;, &quot;.mana/migration-conflicts/267-fix-native-imp-worker-openai-route-failure-when-sp.md.txt&quot;, &quot;.mana/migration-conflicts/27-native-mana-tool-overhaul-background-runs-lightwei.md.txt&quot;, &quot;.mana/migration-conflicts/270-make-uu-install-support-active-shell-binary-repair.md.txt&quot;, &quot;.mana/migration-conflicts/270.1-make-uu-install-complete-the-active-shell-imp-upgr.md.txt&quot;, &quot;.mana/migration-conflicts/271-harden-spawn-and-mana-tool-termination-so-closespa.md.txt&quot;, &quot;.mana/migration-conflicts/271.1-diagnose-hang-paths-in-imp-spawn-and-mana-closetoo.md.txt&quot;, &quot;.mana/migration-conflicts/273-make-pi-typescript-extensions-importable-into-imp.md.txt&quot;, &quot;.mana/migration-conflicts/275-rethink-imp-tui-tool-call-presentation-and-sidebar.md.txt&quot;, &quot;.mana/migration-conflicts/44-rethink-imp-extensions-as-guest-runtimes-with-opti.md.txt&quot;, &quot;.mana/migration-conflicts/44.1-plan-phased-implementation-of-imp-native-code-inte.md.txt&quot;, &quot;.mana/migration-conflicts/45-explore-ast-backed-symbolic-plan-layer-for-imp.md.txt&quot;, &quot;.mana/migration-conflicts/51-easy-fix-impmana-gaps-triaged-from-repo-scan.md.txt&quot;, &quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;README.md&quot;, &quot;crates/imp-cli/auth.json&quot;, &quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/Cargo.toml&quot;, &quot;crates/imp-core/skills/lua-tools/SKILL.md&quot;, &quot;crates/imp-core/skills/writing-skills/REFERENCE.md&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/import.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/sdk.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/bun_runner.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/discovery.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/pi_compat.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/schema.rs&quot;], &quot;insertions&quot;: 21, &quot;deletions&quot;: 29398, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;mostly README&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;, &quot;very high churn (29419 lines)&quot;]}, {&quot;sha&quot;: &quot;34f8be6671f5091d82792eff6ab9bba4ee34f6df&quot;, &quot;short&quot;: &quot;34f8be6&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T12:27:45-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.gitleaks.toml&quot;, &quot;Cargo.toml&quot;, &quot;crates/imp-cli/.gitignore&quot;, &quot;crates/imp-cli/Cargo.toml&quot;], &quot;insertions&quot;: 15, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;2c50e9633a829dec714836848a9faa3da14c7014&quot;, &quot;short&quot;: &quot;2c50e96&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T11:55:43-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.github/workflows/edge.yml&quot;, &quot;.github/workflows/release.yml&quot;], &quot;insertions&quot;: 2, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 13, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;risky subject keyword&quot;, &quot;touches .github/workflows&quot;]}, {&quot;sha&quot;: &quot;d36a3c1142af4797684158f90dc65d1a44357655&quot;, &quot;short&quot;: &quot;d36a3c1&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T10:11:53-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-cli/src/usage_report.rs&quot;, &quot;crates/imp-core/examples/sdk_session.rs&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/error_display.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/personality.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/session.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/web/read.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-lua/src/lib.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/terminal.rs&quot;, &quot;crates/imp-tui/src/views/ask_bar.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/sidebar.rs&quot;, &quot;crates/imp-tui/src/views/startup.rs&quot;, &quot;crates/imp-tui/src/views/tool_output.rs&quot;], &quot;insertions&quot;: 209, &quot;deletions&quot;: 211, &quot;risk_score&quot;: 44, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;moderate churn (420 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;371150fdaca0c02e3140222f84c03c6135153840&quot;, &quot;short&quot;: &quot;371150f&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T09:19:21-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;README.md&quot;], &quot;insertions&quot;: 124, &quot;deletions&quot;: 311, &quot;risk_score&quot;: 7, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;moderate churn (435 lines)&quot;, &quot;mostly README&quot;, &quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;b472eadd5b6afbe7a4a06aa7ec603043031f578b&quot;, &quot;short&quot;: &quot;b472ead&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T07:52:46-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;README.md&quot;], &quot;insertions&quot;: 21, &quot;deletions&quot;: 21, &quot;risk_score&quot;: 12, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;mostly README&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;42634dbe7b8171671fcef2063b765fe8284f93c0&quot;, &quot;short&quot;: &quot;42634db&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-17T18:30:33-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.gitignore&quot;, &quot;AGENTS.md&quot;, &quot;CHANGELOG.md&quot;, &quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;Cargo.workspace.toml&quot;, &quot;LICENSE&quot;, &quot;README.md&quot;, &quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/Cargo.toml&quot;, &quot;crates/imp-core/examples/tool_surface_live.rs&quot;, &quot;crates/imp-core/skills/writing-skills/REFERENCE.md&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/loop_policy.rs&quot;, &quot;crates/imp-core/src/agent/loop_state.rs&quot;, &quot;crates/imp-core/src/agent/mana_loop.rs&quot;, &quot;crates/imp-core/src/{agent.rs =&gt; agent/mod.rs}&quot;, &quot;crates/imp-core/src/agent/recovery.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/agent/turn_assessment.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/config.rs&quot;, &quot;crates/imp-core/src/context_prefill.rs&quot;, &quot;crates/imp-core/src/contracts.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/guardrails.rs&quot;, &quot;crates/imp-core/src/imp_session.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_next/ledger.rs&quot;, &quot;crates/imp-core/src/mana_next/mod.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/mana_run_state.rs&quot;, &quot;crates/imp-core/src/mana_worker.rs&quot;, &quot;crates/imp-core/src/policy.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/resources.rs&quot;, &quot;crates/imp-core/src/retry.rs&quot;, &quot;crates/imp-core/src/roles.rs&quot;, &quot;crates/imp-core/src/run_evidence.rs&quot;, &quot;crates/imp-core/src/session.rs&quot;, &quot;crates/imp-core/src/storage.rs&quot;, &quot;crates/imp-core/src/system_prompt.rs&quot;, &quot;crates/imp-core/src/tools/ask.rs&quot;, &quot;crates/imp-core/src/tools/bash.rs&quot;, &quot;crates/imp-core/src/tools/edit.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/git.rs&quot;, &quot;crates/imp-core/src/tools/imp.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/memory.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/tools/multi_edit.rs&quot;, &quot;crates/imp-core/src/tools/read.rs&quot;, &quot;crates/imp-core/src/tools/scan/kotlin.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/session_search.rs&quot;, &quot;crates/imp-core/src/tools/shell.rs&quot;, &quot;crates/imp-core/src/tools/web/github.rs&quot;, &quot;crates/imp-core/src/tools/web/mod.rs&quot;, &quot;crates/imp-core/src/tools/web/read.rs&quot;, &quot;crates/imp-core/src/tools/web/search.rs&quot;, &quot;crates/imp-core/src/tools/web/types.rs&quot;, &quot;crates/imp-core/src/tools/web/youtube.rs&quot;, &quot;crates/imp-core/src/tools/worktree.rs&quot;, &quot;crates/imp-core/src/tools/write.rs&quot;, &quot;crates/imp-core/src/trace.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/ui.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-gui/Cargo.toml&quot;, &quot;crates/imp-gui/README.md&quot;, &quot;crates/imp-gui/src/lib.rs&quot;, &quot;crates/imp-gui/src/main.rs&quot;, &quot;crates/imp-llm/Cargo.toml&quot;, &quot;crates/imp-llm/src/lib.rs&quot;, &quot;crates/imp-llm/src/provider.rs&quot;, &quot;crates/imp-llm/src/providers/anthropic.rs&quot;, &quot;crates/imp-llm/src/providers/openai.rs&quot;, &quot;crates/imp-lua/src/bridge.rs&quot;, &quot;crates/imp-lua/src/lib.rs&quot;, &quot;crates/imp-lua/src/loader.rs&quot;, &quot;crates/imp-lua/src/sandbox.rs&quot;, &quot;crates/imp-tui/Cargo.toml&quot;, &quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/event_source.rs&quot;, &quot;crates/imp-tui/src/keybindings.rs&quot;, &quot;crates/imp-tui/src/lib.rs&quot;, &quot;crates/imp-tui/src/terminal.rs&quot;, &quot;crates/imp-tui/src/tui_interface.rs&quot;, &quot;crates/imp-tui/src/turn_tracker.rs&quot;, &quot;crates/imp-tui/src/views/ask_bar.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;, &quot;crates/imp-tui/src/views/command_palette.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/file_finder.rs&quot;, &quot;crates/imp-tui/src/views/mana_navigator.rs&quot;, &quot;crates/imp-tui/src/views/mod.rs&quot;, &quot;crates/imp-tui/src/views/session_picker.rs&quot;, &quot;crates/imp-tui/src/views/settings.rs&quot;, &quot;crates/imp-tui/src/views/sidebar.rs&quot;, &quot;crates/imp-tui/src/views/startup.rs&quot;, &quot;crates/imp-tui/src/views/tool_output.rs&quot;, &quot;crates/imp-tui/src/views/tools.rs&quot;, &quot;docs/autonomy-modes.md&quot;, &quot;docs/design/lua-programmatic-interactions.md&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;, &quot;docs/mana-next-compatibility-adapter.md&quot;, &quot;docs/mana-next-examples.md&quot;, &quot;docs/mana-next-migration-test-plan.md&quot;, &quot;docs/mana-next-runtime-event-mapping.md&quot;, &quot;docs/mana-next-storage-strategy.md&quot;, &quot;docs/mana-next-ux.md&quot;, &quot;docs/mana-next-workflow-ledger.md&quot;, &quot;docs/reference-monitor-policy.md&quot;, &quot;docs/run-evidence.md&quot;, &quot;docs/trace-and-evidence-format.md&quot;, &quot;docs/trust-labels-and-provenance.md&quot;, &quot;docs/tui-workflow-wireframes.md&quot;, &quot;docs/verification-gates.md&quot;, &quot;docs/worktree-auto.md&quot;, &quot;imp-gui-wireframe.html&quot;], &quot;insertions&quot;: 39025, &quot;deletions&quot;: 5869, &quot;risk_score&quot;: 56, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;mostly CHANGELOG&quot;, &quot;mostly README&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/mana_worker&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-llm/src/providers&quot;, &quot;touches crates/imp-tui/src/app&quot;, &quot;touches crates/imp-tui/src/event_source&quot;, &quot;very high churn (44894 lines)&quot;]}, {&quot;sha&quot;: &quot;eb3f46fb52a4b11228cf0df7d889a2d40e845980&quot;, &quot;short&quot;: &quot;eb3f46f&quot;, &quot;subject&quot;: &quot;Use published mana crates for release build&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-01T15:40:38-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;crates/imp-core/Cargo.toml&quot;], &quot;insertions&quot;: 8, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;9e6cd9c85b0da3cc2b93bed18a476e265ad719bb&quot;, &quot;short&quot;: &quot;9e6cd9c&quot;, &quot;subject&quot;: &quot;Clean release branch artifacts&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-01T14:12:24-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;.gitignore&quot;, &quot;.mana/.3-set-up-harbor-adapter-and-terminal-bench-20-runner.md&quot;, &quot;.mana/.5-add-safe-automatic-context-compaction-for-long-run.md&quot;, &quot;.mana/.5.1-add-disabled-by-default-auto-compaction-config-sca.md&quot;, &quot;.mana/.6-hardening-pass-reduce-bugs-and-contract-mismatches.md&quot;, &quot;.mana/.6.6-enforce-lua-extension-capability-boundaries.md&quot;, &quot;.mana/.6.7-propagate-cancellation-into-active-tool-execution.md&quot;, &quot;.mana/.6.8-align-diff-tool-registration-with-mode-contracts.md&quot;, &quot;.mana/.9-upgrade-imp-mana-authoring-prompt-contract-for-orc.md&quot;, &quot;.mana/.gitignore&quot;, &quot;.mana/21-imp-efficiency-smarter-tool-output-truncation.md&quot;, &quot;.mana/245.1-define-manaimp-contract-implications-of-file-nativ.md&quot;, &quot;.mana/245.1.1-define-vnext-manaimp-subagent-handoff-packet-for-o.md&quot;, &quot;.mana/248-comprehensive-imp-uiux-review-upgrade-and-polish-a.md&quot;, &quot;.mana/248.14-implement-restrained-ansi-emphasis-for-shell-typog.md&quot;, &quot;.mana/248.16.5-create-svg-wireframes-for-candidate-imp-tui-layout.md&quot;, &quot;.mana/248.16.7-revise-imp-tui-wireframes-and-core-memo-after-user.md&quot;, &quot;.mana/248.17-design-terminal-emulator-native-coding-agent-cockp.md&quot;, &quot;.mana/248.18-harden-and-humanize-imp-error-streaming-across-pro.md&quot;, &quot;.mana/248.18.1-extract-shared-imp-core-streamed-error-normalizati.md&quot;, &quot;.mana/248.18.2-harden-imp-core-partial-stream-and-silent-eof-diag.md&quot;, &quot;.mana/248.18.3-design-stable-machine-facing-streamed-error-envelo.md&quot;, &quot;.mana/248.7-plan-shared-uxruntime-seams-for-shell-tui-and-view.md&quot;, &quot;.mana/248.9-capture-and-sequence-real-user-feedback-on-the-new.md&quot;, &quot;.mana/249-reduce-duplicate-verbose-mana-change-narration-in.md&quot;, &quot;.mana/250-shape-getimpdev-landing-page-direction-and-impleme.md&quot;, &quot;.mana/254-fresh-smoke-trial-for-native-imp-run-on-an-isolate.md&quot;, &quot;.mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md&quot;, &quot;.mana/257-draft-imp-ontologymd-for-shared-featureruntime-lan.md&quot;, &quot;.mana/259-audit-panic-usage-and-detached-task-failure-policy.md&quot;, &quot;.mana/263-audit-and-isolate-library-level-stderr-writes-that.md&quot;, &quot;.mana/263.2-classify-mana-core-stderr-writes-for-embedded-risk.md&quot;, &quot;.mana/264-normalize-imp-storage-topology-for-sessions-config.md&quot;, &quot;.mana/264.1-audit-current-imp-durable-storage-surfaces-and-pat.md&quot;, &quot;.mana/264.2-specify-normalized-imp-storage-contract-and-migrat.md&quot;, &quot;.mana/264.3.1-add-shared-imp-core-storage-path-module-for-canoni.md&quot;, &quot;.mana/264.3.2-migrate-config-auth-session-and-session-search-cal.md&quot;, &quot;.mana/264.3.3-migrate-instruction-discovery-to-canonical-impagen.md&quot;, &quot;.mana/264.3.4-implement-non-destructive-migration-from-legacy-im.md&quot;, &quot;.mana/264.4-audit-and-fix-imp-session-index-lifecycle-wiring-f.md&quot;, &quot;.mana/264.6-decide-canonical-imp-filesystem-roots-for-global-a.md&quot;, &quot;.mana/264.7-specify-precedence-and-merge-rules-between-imp-and.md&quot;, &quot;.mana/264.8-specify-migration-from-xdgmacos-legacy-paths-into.md&quot;, &quot;.mana/266-cross-codebase-review-compare-imp-and-hermes-agent.md&quot;, &quot;.mana/266.1-design-adoption-path-provider-resilience-and-auth.md&quot;, &quot;.mana/266.2-design-adoption-path-session-recall-memory-and-con.md&quot;, &quot;.mana/266.3-design-adoption-path-extension-seams-and-product-s.md&quot;, &quot;.mana/266.4-imp-vnext-ranked-roadmap-and-phased-execution-plan.md&quot;, &quot;.mana/266.4.7-phase-5-epic-selective-later-product-surface-expan.md&quot;, &quot;.mana/267-adopt-highest-value-product-lessons-from-opencode.md&quot;, &quot;.mana/268.1-diagnose-native-imp-mana-tool-divergence-from-cli.md&quot;, &quot;.mana/27-improve-mana-pool-competitive-grade-dispatch-engin.md&quot;, &quot;.mana/27.14-define-attempt-scoped-autonomy-observation-record.md&quot;, &quot;.mana/27.2-imp-ui-compact-mana-statusprogress-surface.md&quot;, &quot;.mana/271-add-native-youtube-video-interpretation-support-to.md&quot;, &quot;.mana/271.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/271.2-harden-imp-spawn-and-mana-closetool-execution-agai.md&quot;, &quot;.mana/272-add-native-video-context-ingestion-architecture-fo.md&quot;, &quot;.mana/272.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/272.2-design-richer-video-interpretation-beyond-transcri.md&quot;, &quot;.mana/273-diagnose-and-harden-kimi-code-oauth-model-routing.md&quot;, &quot;.mana/273.5-sprint-import-and-execute-pi-typescript-extensions.md&quot;, &quot;.mana/273.5.10-prove-bun-ts-adapter-against-local-pi-color-palett.md&quot;, &quot;.mana/273.5.11-add-official-pi-dynamic-tools-compatibility-fixtur.md&quot;, &quot;.mana/273.5.12-define-sprint-1-typescriptpi-extension-support-bou.md&quot;, &quot;.mana/273.5.13-probe-dependency-bearing-pi-extension-compatibilit.md&quot;, &quot;.mana/273.5.4-normalize-typeboxjson-schemas-from-typescript-exte.md&quot;, &quot;.mana/275-assess-and-sequence-next-llm-oauth-provider-integr.md&quot;, &quot;.mana/275.10-inventory-pi-and-imp-provideroauth-surfaces.md&quot;, &quot;.mana/275.11-sequence-pi-provideroauth-parity-implementation.md&quot;, &quot;.mana/275.6-assess-pi-google-antigravity-provider-route-for-im.md&quot;, &quot;.mana/275.9-research-unofficial-cursor-provider-support-for-im.md&quot;, &quot;.mana/276-investigate-and-harden-tui-esc-cancellation-for-hu.md&quot;, &quot;.mana/277-fix-imp-tui-clean-ui-corruption-and-string-join-ov.md&quot;, &quot;.mana/278-fix-inspector-mode-interaction-model.md&quot;, &quot;.mana/28.1-make-imp-run-the-canonical-mana-worker-runtime-whi.md&quot;, &quot;.mana/28.5.1-patch-imp-system-prompt-with-mana-first-planning-d.md&quot;, &quot;.mana/28.5.6-implement-turn-scoped-mana-review-packet-aggregati.md&quot;, &quot;.mana/28.5.7-render-between-turn-mana-review-packets-across-imp.md&quot;, &quot;.mana/28.5.7.1-add-shared-imp-core-turnmanadelta-renderer-and-man.md&quot;, &quot;.mana/28.5.7.2-render-compact-between-turn-mana-block-and-textual.md&quot;, &quot;.mana/28.5.7.3-render-between-turn-mana-review-packets-in-imp-cli.md&quot;, &quot;.mana/28.5.7.4-add-shared-manareviewmode-config-and-presentation.md&quot;, &quot;.mana/28.5.7.5-wire-imp-tui-compact-widget-tray-block-and-sidebar.md&quot;, &quot;.mana/280-review-project-gaps-that-would-make-imp-stronger-t.md&quot;, &quot;.mana/280.1-run-dirac-evals-with-imp-using-gemini-secret.md&quot;, &quot;.mana/280.2-adopt-dirac-inspired-code-intelligence-and-precise.md&quot;, &quot;.mana/280.2.1.1-decide-migration-safe-naming-strategy-for-imp-scan.md&quot;, &quot;.mana/280.2.2-implement-read-oriented-symbol-extraction-and-skel.md&quot;, &quot;.mana/280.2.3-add-anchor-backed-read-and-stale-safe-edit-flow-to.md&quot;, &quot;.mana/280.2.4-implement-edit-transaction-batching-with-combined.md&quot;, &quot;.mana/282-design-native-scoped-secret-injection-for-command.md&quot;, &quot;.mana/285-verify-installed-imp-binary-includes-latest-secret.md&quot;, &quot;.mana/290-complete-imp-codebase-quality-audit.md&quot;, &quot;.mana/290.1-split-imp-tui-apprs-by-responsibility.md&quot;, &quot;.mana/290.2-split-imp-core-agentrs-into-focused-runtime-module.md&quot;, &quot;.mana/290.3-split-imp-cli-librs-into-command-modules.md&quot;, &quot;.mana/290.4-split-native-mana-tool-implementation-into-focused.md&quot;, &quot;.mana/291-rewrite-imp-readme-around-product-features-mana-an.md&quot;, &quot;.mana/31.2-add-guardrail-config-types-and-profile-selection-t.md&quot;, &quot;.mana/31.3-integrate-guardrails-into-the-imp-system-prompt-an.md&quot;, &quot;.mana/31.4-add-the-initial-zig-guardrail-profile-and-document.md&quot;, &quot;.mana/33-chat-view-replace-duplicated-animation-logic-with.md&quot;, &quot;.mana/34-sidebar-detail-header-use-spinnerframe-and-respect.md&quot;, &quot;.mana/35-editor-remove-dead-tick-and-animationlevel-params.md&quot;, &quot;.mana/36-animation-config-reconcile-minimal-namingdocs-afte.md&quot;, &quot;.mana/37-add-first-class-usage-accounting-and-reporting-to.md&quot;, &quot;.mana/37.5-test-and-document-imp-usage-accountingreporting.md&quot;, &quot;.mana/41-anthropic-api-parity-adopt-claude-code-patterns-fo.md&quot;, &quot;.mana/44-define-memory-and-code-intelligence-architecture-f.md&quot;, &quot;.mana/44.1-author-guest-runtime-extension-substrate-proposal.md&quot;, &quot;.mana/44.1.10-implement-documentworkspace-symbols-with-ast-first.md&quot;, &quot;.mana/44.1.11-implement-hover-and-signature-help-on-the-phase-1.md&quot;, &quot;.mana/44.1.12-unify-code-intelligence-diagnostic-summaries-with.md&quot;, &quot;.mana/44.1.14-evaluate-whether-repeated-evidence-promotion-flows.md&quot;, &quot;.mana/44.1.5-plan-guarded-write-oriented-semantic-actions-and-p.md&quot;, &quot;.mana/44.1.5.5-specify-semantic-write-execution-contract-for-prev.md&quot;, &quot;.mana/44.1.6-sequence-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.6.1-define-shared-normalization-envelopes-for-read-ori.md&quot;, &quot;.mana/44.1.6.2-plan-diagnostics-plus-ast-alignment-for-the-first.md&quot;, &quot;.mana/44.1.6.3-plan-document-symbols-and-go-to-definition-over-th.md&quot;, &quot;.mana/44.1.6.4-plan-references-and-workspace-symbol-browsing-for.md&quot;, &quot;.mana/44.1.6.5-plan-hover-and-signature-enrichment-after-core-rea.md&quot;, &quot;.mana/44.1.7-roll-out-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.8-normalize-read-oriented-code-intelligence-queryres.md&quot;, &quot;.mana/44.1.9-implement-phase-1-diagnostics-go-to-definition-and.md&quot;, &quot;.mana/44.3-translate-guest-runtime-design-into-phased-impleme.md&quot;, &quot;.mana/45-tower-rebuild-around-explicit-contracts-durable-le.md&quot;, &quot;.mana/45.10.5-update-docs-for-mana-platform-substrate-and-imp-pr.md&quot;, &quot;.mana/45.11-capture-near-term-imp-execution-lanes-under-the-im.md&quot;, &quot;.mana/45.11.1-resolve-consequential-defaults-for-near-term-imp-i.md&quot;, &quot;.mana/45.11.1.1-clarify-whether-native-rust-not-lua-applies-to-imp.md&quot;, &quot;.mana/45.11.1.2-sequence-near-term-imp-implementation-order-from-s.md&quot;, &quot;.mana/45.4-phase-3-introduce-runner-protocol-and-local-adapte.md&quot;, &quot;.mana/45.4.2-plan-the-first-imp-local-runner-adapter-that-consu.md&quot;, &quot;.mana/45.4.4-plan-the-cutover-from-current-imp-run-plus-mana-ru.md&quot;, &quot;.mana/45.5-phase-4-rebuild-imp-around-stable-workerruntime-se.md&quot;, &quot;.mana/45.5.1-map-imp-core-hotspots-into-target-runtime-context.md&quot;, &quot;.mana/45.5.3-write-a-compact-imp-decomposition-order-for-post-c.md&quot;, &quot;.mana/45.7-phase-6-harden-policy-isolation-and-migration-surf.md&quot;, &quot;.mana/45.7.4-evaluate-whether-imp-should-add-a-native-gitrepo-t.md&quot;, &quot;.mana/46-broaden-imp-attention-beyond-toolsprompting-under.md&quot;, &quot;.mana/46.1-reconcile-long-session-runtime-safety-gaps-and-tur.md&quot;, &quot;.mana/46.2-reconcile-user-visible-discoverability-and-ux-brea.md&quot;, &quot;.mana/46.2.1-surface-usage-reporting-in-the-tui-commandhelpstar.md&quot;, &quot;.mana/47-rebuild-imp-around-explicit-runtime-boundaries-pro.md&quot;, &quot;.mana/47.1-contracts-and-ownership-boundary-for-mana-imp-rebu.md&quot;, &quot;.mana/47.6-sequence-the-imp-rebuild-as-an-incremental-migrati.md&quot;, &quot;.mana/50-define-cli-first-operator-surface-for-imp-with-tui.md&quot;, &quot;.mana/50.10-implement-guided-cli-parity-flows-for-settings-per.md&quot;, &quot;.mana/50.10.1-implement-terminal-native-imp-settings-flow-for-cl.md&quot;, &quot;.mana/50.10.1.2-let-imp-chat-no-tools-reach-the-shell-without-prov.md&quot;, &quot;.mana/50.10.2-implement-terminal-native-imp-personality-flow-for.md&quot;, &quot;.mana/50.11-implement-first-shell-to-view-handoff-for-sessions.md&quot;, &quot;.mana/50.11.2-align-imp-chat-view-handoff-with-explicit-imp-view.md&quot;, &quot;.mana/50.12-flip-plain-imp-to-imp-chat-after-shell-readiness-g.md&quot;, &quot;.mana/50.13-plan-extraction-of-shared-fullscreen-consumed-runt.md&quot;, &quot;.mana/50.14-specify-the-shared-imp-ui-request-and-runtime-even.md&quot;, &quot;.mana/50.16-follow-on-cli-native-affordance-stack-after-505-se.md&quot;, &quot;.mana/50.16.1-define-stable-human-vs-machine-output-modes-across.md&quot;, &quot;.mana/50.16.2-plan-cli-first-checkpoint-productization-after-out.md&quot;, &quot;.mana/50.16.3-plan-visible-cli-first-planning-artifacts-and-exec.md&quot;, &quot;.mana/50.16.4-plan-first-class-approval-policy-layer-for-cli-fir.md&quot;, &quot;.mana/50.16.5-surface-session-browsing-and-session-search-as-fir.md&quot;, &quot;.mana/50.16.5.1-audit-and-reconcile-imp-session-storage-and-sessio.md&quot;, &quot;.mana/50.16.6-plan-detachedbackground-local-execution-after-cli.md&quot;, &quot;.mana/50.17-define-stable-human-vs-machine-output-contracts-fo.md&quot;, &quot;.mana/50.18-define-cli-first-session-browsing-and-sessionsearc.md&quot;, &quot;.mana/50.19-define-stable-imp-human-vs-machine-output-contract.md&quot;, &quot;.mana/50.20-plan-first-cli-first-checkpoint-productization-ove.md&quot;, &quot;.mana/50.21-specify-visible-planning-artifacts-and-checklist-b.md&quot;, &quot;.mana/50.22-specify-the-first-visible-planning-workflow-and-pl.md&quot;, &quot;.mana/50.23-specify-cli-first-approval-policy-and-blocked-stat.md&quot;, &quot;.mana/50.24-define-the-first-cli-first-approval-policy-surface.md&quot;, &quot;.mana/50.25-specify-detachedbackground-local-execution-contrac.md&quot;, &quot;.mana/50.26-define-the-first-local-detachedbackground-executio.md&quot;, &quot;.mana/50.6-design-the-cli-first-interactive-shell-path-for-im.md&quot;, &quot;.mana/50.9-implement-the-first-cli-first-proving-slice-with-e.md&quot;, &quot;.mana/51.6.1-audit-current-mana-core-embedding-surface-against.md&quot;, &quot;.mana/65-root-mana-currently-lists-child-513-but-direct-sho.md&quot;, &quot;.mana/69-imp-cli-no-longer-contains-duplicate-mana-unit-loa.md&quot;, &quot;.mana/73-code-intelligence-outputs-are-transient-by-default.md&quot;, &quot;.mana/81-design-imp-native-delegation-tool-around-imp-run-a.md&quot;, &quot;.mana/81.10-define-codemap-backed-context-seam-for-imp-run-and.md&quot;, &quot;.mana/82-assess-gpt-54-pro-support-through-openai-chatgpt-o.md&quot;, &quot;.mana/82.2-add-gpt-54-pro-to-imp-model-registry-only-after-oa.md&quot;, &quot;.mana/83-harden-imp-tui-text-box-cursor-and-bounds-handling.md&quot;, &quot;.mana/RULES.md&quot;, &quot;.mana/archive/2026/03/.2-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/16-imp-core-hardening-production-ready-agent-engine.md&quot;, &quot;.mana/archive/2026/03/16.1-wire-config-agent-agentbuilder-thresholds-hooks-re.md&quot;, &quot;.mana/archive/2026/03/16.2-tool-argument-validation-json-schema-before-execut.md&quot;, &quot;.mana/archive/2026/03/16.3-llm-retry-with-exponential-backoff-and-jitter.md&quot;, &quot;.mana/archive/2026/03/16.4-loop-detection-prevent-infinite-tool-call-retry-lo.md&quot;, &quot;.mana/archive/2026/03/16.5-file-not-found-suggestions-with-levenshtein-rankin.md&quot;, &quot;.mana/archive/2026/03/16.6-auto-resume-after-compaction-re-queue-original-pro.md&quot;, &quot;.mana/archive/2026/03/16.7-file-read-tracking-and-staleness-detection.md&quot;, &quot;.mana/archive/2026/03/16.8-file-version-history-pre-edit-snapshots-for-rollba.md&quot;, &quot;.mana/archive/2026/03/17-imp-efficiency-enable-prompt-caching.md&quot;, &quot;.mana/archive/2026/03/19-imp-efficiency-in-session-file-content-cache.md&quot;, &quot;.mana/archive/2026/03/20-imp-efficiency-parallelize-grep-block-search-with.md&quot;, &quot;.mana/archive/2026/03/229-imp-rust-coding-agent-engine.md&quot;, &quot;.mana/archive/2026/03/229.1-workspace-setup-imp-llm-types.md&quot;, &quot;.mana/archive/2026/03/229.10-imp-llm-anthropic-oauth.md&quot;, &quot;.mana/archive/2026/03/229.11-imp-core-hook-system.md&quot;, &quot;.mana/archive/2026/03/229.12-imp-core-tree-sitter-tools-probesearch-probeextrac.md&quot;, &quot;.mana/archive/2026/03/229.13-imp-core-config-resource-discovery.md&quot;, &quot;.mana/archive/2026/03/229.14-imp-core-system-prompt-assembly.md&quot;, &quot;.mana/archive/2026/03/229.15-imp-lua-lua-extension-runtime.md&quot;, &quot;.mana/archive/2026/03/229.16-imp-core-shell-tool-loader.md&quot;, &quot;.mana/archive/2026/03/229.17-imp-tui-ratatui-interactive-mode.md&quot;, &quot;.mana/archive/2026/03/229.18-imp-cli-binary-entry-point.md&quot;, &quot;.mana/archive/2026/03/229.2-imp-llm-anthropic-provider.md&quot;, &quot;.mana/archive/2026/03/229.3-imp-core-tool-trait-file-tools-read-write-edit-mul.md&quot;, &quot;.mana/archive/2026/03/229.4-imp-core-bash-grep-find-tools.md&quot;, &quot;.mana/archive/2026/03/229.5-imp-core-ask-diff-tools.md&quot;, &quot;.mana/archive/2026/03/229.6-imp-core-agent-loop.md&quot;, &quot;.mana/archive/2026/03/229.7-imp-core-session-manager.md&quot;, &quot;.mana/archive/2026/03/229.8-imp-core-context-management-observation-masking-co.md&quot;, &quot;.mana/archive/2026/03/229.9-imp-llm-openai-google-providers.md&quot;, &quot;.mana/archive/2026/03/23-learning-loop-agent-curated-memory-skill-managemen.md&quot;, &quot;.mana/archive/2026/03/23.1-system-prompt-layer-6-wire-memory-into-prompt-asse.md&quot;, &quot;.mana/archive/2026/03/23.2-memory-store-and-memory-tool.md&quot;, &quot;.mana/archive/2026/03/23.3-skill-manage-tool-agent-creates-patches-and-delete.md&quot;, &quot;.mana/archive/2026/03/23.4-learning-nudges-system-prompt-text-and-onagentend.md&quot;, &quot;.mana/archive/2026/03/23.5-session-index-with-fts5-full-text-search.md&quot;, &quot;.mana/archive/2026/03/23.6-session-search-tool.md&quot;, &quot;.mana/archive/2026/03/24-tui-ux-overhaul-information-density-summaries-inte.md&quot;, &quot;.mana/archive/2026/03/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/03/24.2-progress-indicator-in-status-bar-during-streaming.md&quot;, &quot;.mana/archive/2026/03/24.3-per-tool-call-expandcollapse-and-auto-expand-error.md&quot;, &quot;.mana/archive/2026/03/24.4-turn-end-summary-with-file-change-tracking.md&quot;, &quot;.mana/archive/2026/03/24.5-visual-separation-of-tool-activity-from-assistant.md&quot;, &quot;.mana/archive/2026/03/24.6-editor-polish-placeholder-model-indicator-keybindi.md&quot;, &quot;.mana/archive/2026/03/24.7-fix-context-window-tracking-use-actual-conversatio.md&quot;, &quot;.mana/archive/2026/03/24.8-approval-flow-wire-userinterface-for-tool-confirma.md&quot;, &quot;.mana/archive/2026/03/25-multi-provider-llm-support-with-data-driven-welcom.md&quot;, &quot;.mana/archive/2026/03/25.1-provider-metadata-registry-auth-generalization.md&quot;, &quot;.mana/archive/2026/03/25.2-openai-compatible-chat-completions-provider.md&quot;, &quot;.mana/archive/2026/03/25.3-add-builtin-models-for-new-providers.md&quot;, &quot;.mana/archive/2026/03/25.4-data-driven-welcome-flow.md&quot;, &quot;.mana/archive/2026/03/25.5-generalize-cli-login-for-all-providers.md&quot;, &quot;.mana/archive/2026/03/26-fix-imp-tui-compile-errors-around-toolcallorder-re.md&quot;, &quot;.mana/archive/2026/03/27.1-imp-core-mana-tool-add-native-orchestration-action.md&quot;, &quot;.mana/archive/2026/03/31-add-configurable-engineering-guardrails-to-imp.md&quot;, &quot;.mana/archive/2026/03/37.1-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/37.2-persist-canonical-usage-entries-in-imp-core-sessio.md&quot;, &quot;.mana/archive/2026/03/37.3-unify-usage-persistence-across-imp-execution-paths.md&quot;, &quot;.mana/archive/2026/03/37.4-add-imp-usage-reporting-commands-and-export.md&quot;, &quot;.mana/archive/2026/04/.10-define-clean-mana-vs-imp-boundary-and-memory-conso.md&quot;, &quot;.mana/archive/2026/04/.10.1-define-imp-memory-layer-architecture-and-mana-ownership-boundaries.md&quot;, &quot;.mana/archive/2026/04/.10.2-design-a-mana-wiki-schema-and-knowledge-maintenance-workflow.md&quot;, &quot;.mana/archive/2026/04/.10.3-strengthen-mana-first-prompt-doctrine-for-durable-planning.md&quot;, &quot;.mana/archive/2026/04/.10.4-design-mana-aware-runtime-context-read-path-for-prompt-assembly.md&quot;, &quot;.mana/archive/2026/04/.10.5-design-inline-mana-state-and-knowledge-surfaces-for-imp-runtime.md&quot;, &quot;.mana/archive/2026/04/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/04/266.4.3.4-fix-stale-secret-metadata-and-missing-keychain-dia.md&quot;, &quot;.mana/archive/2026/04/27.4-imp-promptingtool-guidance-prefer-native-mana-tool.md&quot;, &quot;.mana/archive/2026/04/272-add-kimi-model-compatibility-and-fix-ctrll-model-p.md&quot;, &quot;.mana/archive/2026/04/274-audit-and-simplify-imp-core-config-module.md&quot;, &quot;.mana/archive/2026/04/28-surface-built-in-features-already-implemented-in-i.md&quot;, &quot;.mana/archive/2026/04/28.1.1-specify-the-strengthened-imp-run-worker-contract-a.md&quot;, &quot;.mana/archive/2026/04/28.1.2-implement-reusable-imp-side-mana-unit-worker-runti.md&quot;, &quot;.mana/archive/2026/04/28.1.3-integrate-mana-run-with-the-strengthened-imp-run-w.md&quot;, &quot;.mana/archive/2026/04/28.1.5-fix-native-imp-delegate-worker-defaults-for-openai.md&quot;, &quot;.mana/archive/2026/04/28.1.5-make-imps-native-mana-tool-the-clear-first-class-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.2-fix-direct-imp-run-codexopenai-worker-request-defa.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.2-extract-shared-model-first-runtime-connection-reso.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.3-refactor-headless-worker-auth-to-normalize-empty-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.4-clarify-imp-to-imp-tool-vocabulary-and-align-docs.md&quot;, &quot;.mana/archive/2026/04/29.3-add-recent-session-previews-to-the-imp-startup-pan.md&quot;, &quot;.mana/archive/2026/04/29.4-add-context-aware-quickstart-guidance-and-health-s.md&quot;, &quot;.mana/archive/2026/04/29.6.1-implement-native-mana-scope-targeting-in-imp-tool.md&quot;, &quot;.mana/archive/2026/04/29.6.2-implement-safe-partial-mana-update-semantics-in-im.md&quot;, &quot;.mana/archive/2026/04/29.6.3-implement-append-style-mana-actions-for-conversati.md&quot;, &quot;.mana/archive/2026/04/30-render-compact-widgetstatus-surfaces-already-suppo.md&quot;, &quot;.mana/archive/2026/04/31.1-write-the-engineering-guardrails-design-note-for-i.md&quot;, &quot;.mana/archive/2026/04/32-productize-checkpoints-from-imps-existing-file-sna.md&quot;, &quot;.mana/archive/2026/04/32.1-checkpoint-foundation-shared-filehistory-wiring-an.md&quot;, &quot;.mana/archive/2026/04/32.2-checkpoint-persistence-session-custom-records-plus.md&quot;, &quot;.mana/archive/2026/04/32.3-checkpoint-ux-minimal-slash-command-list-and-resto.md&quot;, &quot;.mana/archive/2026/04/42-per-agent-cached-context-assembly-for-mana-dispatc.md&quot;, &quot;.mana/archive/2026/04/47.1.4-implement-the-first-shared-verifier-and-evidence-r.md&quot;, &quot;.mana/index.yaml.old&quot;, &quot;.mana/migration-conflicts/.3-add-secure-generic-credential-storage-and-lua-secr.md.txt&quot;, &quot;.mana/migration-conflicts/267-fix-native-imp-worker-openai-route-failure-when-sp.md.txt&quot;, &quot;.mana/migration-conflicts/27-native-mana-tool-overhaul-background-runs-lightwei.md.txt&quot;, &quot;.mana/migration-conflicts/270-make-uu-install-support-active-shell-binary-repair.md.txt&quot;, &quot;.mana/migration-conflicts/270.1-make-uu-install-complete-the-active-shell-imp-upgr.md.txt&quot;, &quot;.mana/migration-conflicts/271-harden-spawn-and-mana-tool-termination-so-closespa.md.txt&quot;, &quot;.mana/migration-conflicts/271.1-diagnose-hang-paths-in-imp-spawn-and-mana-closetoo.md.txt&quot;, &quot;.mana/migration-conflicts/273-make-pi-typescript-extensions-importable-into-imp.md.txt&quot;, &quot;.mana/migration-conflicts/275-rethink-imp-tui-tool-call-presentation-and-sidebar.md.txt&quot;, &quot;.mana/migration-conflicts/44-rethink-imp-extensions-as-guest-runtimes-with-opti.md.txt&quot;, &quot;.mana/migration-conflicts/44.1-plan-phased-implementation-of-imp-native-code-inte.md.txt&quot;, &quot;.mana/migration-conflicts/45-explore-ast-backed-symbolic-plan-layer-for-imp.md.txt&quot;, &quot;.mana/migration-conflicts/51-easy-fix-impmana-gaps-triaged-from-repo-scan.md.txt&quot;, &quot;.tmp/imp-run-trial/one-shot-print.txt&quot;, &quot;.vibecheck/vibecheck.db&quot;, &quot;.vibecheck/vibecheck.db-shm&quot;, &quot;.vibecheck/vibecheck.db-wal&quot;, &quot;=&quot;, &quot;AGENTS copy.md&quot;, &quot;art.html&quot;, &quot;art.html.bak&quot;, &quot;art.md&quot;, &quot;art_original.html&quot;, &quot;art_test.txt&quot;, &quot;crates/imp-cli/auth.json&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/top_bar.rs&quot;, &quot;draft.html&quot;, &quot;evals/dirac-comparison/tasks/DynamicCache.json&quot;, &quot;evals/dirac-comparison/tasks/IOverlayWidget.json&quot;, &quot;evals/dirac-comparison/tasks/addLogging.json&quot;, &quot;evals/dirac-comparison/tasks/datadict.json&quot;, &quot;evals/dirac-comparison/tasks/extensionswb_service.json&quot;, &quot;evals/dirac-comparison/tasks/latency.json&quot;, &quot;evals/dirac-comparison/tasks/sendRequest.json&quot;, &quot;evals/dirac-comparison/tasks/stoppingcriteria.json&quot;, &quot;gen_art.py&quot;, &quot;tmp-find-django.sh&quot;, &quot;tools/imp-fix-signature.sh&quot;], &quot;insertions&quot;: 22, &quot;deletions&quot;: 30014, &quot;risk_score&quot;: 5, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;very high churn (30036 lines)&quot;]}, {&quot;sha&quot;: &quot;31e1a04ab84b95d91e150b6600bf0f5e4523c3cd&quot;, &quot;short&quot;: &quot;31e1a04&quot;, &quot;subject&quot;: &quot;Build workflow runtime foundations&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T15:31:11-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/loop_state.rs&quot;, &quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/context_prefill.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/imp_session.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/mana_worker.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/tools/ask.rs&quot;, &quot;crates/imp-core/src/tools/bash.rs&quot;, &quot;crates/imp-core/src/tools/edit.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/git.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/memory.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/tools/multi_edit.rs&quot;, &quot;crates/imp-core/src/tools/read.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/session_search.rs&quot;, &quot;crates/imp-core/src/tools/shell.rs&quot;, &quot;crates/imp-core/src/tools/web/mod.rs&quot;, &quot;crates/imp-core/src/tools/worktree.rs&quot;, &quot;crates/imp-core/src/tools/write.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-lua/src/sandbox.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/turn_tracker.rs&quot;, &quot;crates/imp-tui/src/views/command_palette.rs&quot;, &quot;docs/autonomy-modes.md&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;, &quot;docs/reference-monitor-policy.md&quot;, &quot;docs/trace-and-evidence-format.md&quot;, &quot;docs/trust-labels-and-provenance.md&quot;, &quot;docs/verification-gates.md&quot;, &quot;docs/worktree-auto.md&quot;], &quot;insertions&quot;: 8086, &quot;deletions&quot;: 108, &quot;risk_score&quot;: 45, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/mana_worker&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-tui/src/app&quot;, &quot;very high churn (8194 lines)&quot;]}, {&quot;sha&quot;: &quot;424795c9063683de1bce9fee5866bf69028c3599&quot;, &quot;short&quot;: &quot;424795c&quot;, &quot;subject&quot;: &quot;Trace TUI agent startup phases&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T12:26:38-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 72, &quot;deletions&quot;: 15, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;d89bafd65ac2f18f6d453f0be3a57df0e0b7b8c3&quot;, &quot;short&quot;: &quot;d89bafd&quot;, &quot;subject&quot;: &quot;Keep title spinner active during agent startup&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:56:18-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 42, &quot;deletions&quot;: 4, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;4543ff22420bf6fdb6a4e03055ac370499baa6f0&quot;, &quot;short&quot;: &quot;4543ff2&quot;, &quot;subject&quot;: &quot;Animate chat waiting placeholder each tick&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:28:10-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 30, &quot;deletions&quot;: 7, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;ef9ccdd138fd69da5959be53846f883c64d6f8f8&quot;, &quot;short&quot;: &quot;ef9ccdd&quot;, &quot;subject&quot;: &quot;Start TUI agents off the input path&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T08:57:59-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 214, &quot;deletions&quot;: 154, &quot;risk_score&quot;: 7, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;moderate churn (368 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;79665f4209ab43760f14f3f635a74434826c069d&quot;, &quot;short&quot;: &quot;79665f4&quot;, &quot;subject&quot;: &quot;Restore faster title spinner cadence&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:13:19-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 8, &quot;deletions&quot;: 8, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;ceb3ef3abdfe6361fbe6daec3b24ce328d52690c&quot;, &quot;short&quot;: &quot;ceb3ef3&quot;, &quot;subject&quot;: &quot;Use clearer title spinner cadence&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:11:18-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 14, &quot;deletions&quot;: 14, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;98cbab62f34389479859bde907fc5b78ddf3e537&quot;, &quot;short&quot;: &quot;98cbab6&quot;, &quot;subject&quot;: &quot;Reuse rendered tool click map for inspector&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T10:24:33-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;], &quot;insertions&quot;: 67, &quot;deletions&quot;: 16, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;b6c301ec1dcb1b0519bbc0d74883885f14b63a48&quot;, &quot;short&quot;: &quot;b6c301e&quot;, &quot;subject&quot;: &quot;Use spinner for TUI working title&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T10:10:56-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 18, &quot;deletions&quot;: 18, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;2b6ef71be3d20f628223b9be70bd28ce55290892&quot;, &quot;short&quot;: &quot;2b6ef71&quot;, &quot;subject&quot;: &quot;Document TUI workflow wireframes&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T12:24:20-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;docs/tui-workflow-wireframes.md&quot;], &quot;insertions&quot;: 753, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 3, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;high churn (753 lines)&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;79b49633d66ee8280af9682c945cab5425a7c428&quot;, &quot;short&quot;: &quot;79b4963&quot;, &quot;subject&quot;: &quot;Add trace and evidence artifacts&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:26:51-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/storage.rs&quot;, &quot;crates/imp-core/src/trace.rs&quot;, &quot;docs/trace-and-evidence-format.md&quot;], &quot;insertions&quot;: 1397, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 5, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;high churn (1397 lines)&quot;, &quot;mostly docs/&quot;, &quot;touches crates/imp-core/src/agent&quot;]}, {&quot;sha&quot;: &quot;e2dba93ca9660c2a24a6256e750773de30e67601&quot;, &quot;short&quot;: &quot;e2dba93&quot;, &quot;subject&quot;: &quot;Add mana workflow ledger model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:26:27-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_next/ledger.rs&quot;, &quot;crates/imp-core/src/mana_next/mod.rs&quot;, &quot;docs/mana-next-compatibility-adapter.md&quot;, &quot;docs/mana-next-examples.md&quot;, &quot;docs/mana-next-migration-test-plan.md&quot;, &quot;docs/mana-next-runtime-event-mapping.md&quot;, &quot;docs/mana-next-storage-strategy.md&quot;, &quot;docs/mana-next-ux.md&quot;, &quot;docs/mana-next-workflow-ledger.md&quot;], &quot;insertions&quot;: 2578, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 0, &quot;risk_label&quot;: &quot;low&quot;, &quot;risk_reasons&quot;: [&quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;very high churn (2578 lines)&quot;]}, {&quot;sha&quot;: &quot;c483434eba3b7434ae4c6f8739afbceeef9567e2&quot;, &quot;short&quot;: &quot;c483434&quot;, &quot;subject&quot;: &quot;Add workflow contract model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:25:46-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/lib.rs&quot;], &quot;insertions&quot;: 1, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 2, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;0184de68d2fc157f6127826c7e1743799a19d7df&quot;, &quot;short&quot;: &quot;0184de6&quot;, &quot;subject&quot;: &quot;Add workflow contract model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:23:35-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;], &quot;insertions&quot;: 1252, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 15, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;high churn (1254 lines)&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/workflow&quot;]}]</script>
crates/imp-llm/src/oauth/anthropic.rs:19:const CALLBACK_HOST: &str = "127.0.0.1";
crates/imp-llm/src/oauth/anthropic.rs:410:        let server = CallbackServer::bind("127.0.0.1", 0).await.unwrap();
crates/imp-llm/src/oauth/anthropic.rs:424:        let mut client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
crates/imp-llm/src/oauth/anthropic.rs:447:        let server = CallbackServer::bind("127.0.0.1", 0).await.unwrap();
crates/imp-llm/src/oauth/anthropic.rs:458:        let mut client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
crates/imp-llm/src/oauth/anthropic.rs:478:        let server = CallbackServer::bind("127.0.0.1", 0).await.unwrap();
crates/imp-llm/src/oauth/anthropic.rs:491:        let listener = TokioListener::bind("127.0.0.1:0").await.unwrap();
crates/imp-llm/src/oauth/anthropic.rs:532:        let oauth = AnthropicOAuth::with_token_url(format!("http://127.0.0.1:{port}/token"));
crates/imp-llm/src/oauth/anthropic.rs:559:        let oauth = AnthropicOAuth::with_token_url(format!("http://127.0.0.1:{port}/token"));
crates/imp-llm/src/oauth/anthropic.rs:585:        let oauth = AnthropicOAuth::with_token_url(format!("http://127.0.0.1:{port}/token"));
crates/imp-llm/src/oauth/anthropic.rs:614:        let oauth = AnthropicOAuth::with_token_url(format!("http://127.0.0.1:{port}/token"));
crates/imp-llm/src/oauth/anthropic.rs:630:        let oauth = AnthropicOAuth::with_token_url(format!("http://127.0.0.1:{port}/token"));
crates/imp-llm/src/oauth/kimi_code.rs:385:        let listener = TokioListener::bind("127.0.0.1:0").await.unwrap();
crates/imp-llm/src/oauth/kimi_code.rs:423:            format!("http://127.0.0.1:{port}/device"),
crates/imp-llm/src/oauth/kimi_code.rs:424:            format!("http://127.0.0.1:{port}/token"),
crates/imp-llm/src/oauth/kimi_code.rs:447:            format!("http://127.0.0.1:{port}/device"),
crates/imp-llm/src/oauth/kimi_code.rs:448:            format!("http://127.0.0.1:{port}/token"),
crates/imp-llm/src/oauth/kimi_code.rs:470:            format!("http://127.0.0.1:{port}/device"),
crates/imp-llm/src/oauth/kimi_code.rs:471:            format!("http://127.0.0.1:{port}/token"),
crates/imp-llm/src/oauth/chatgpt.rs:19:const CALLBACK_HOST_V4: &str = "127.0.0.1";
crates/imp-llm/src/oauth/chatgpt.rs:469:        let mut client = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
crates/imp-llm/src/oauth/chatgpt.rs:490:        let listener = TokioListener::bind("127.0.0.1:0").await.unwrap();
crates/imp-llm/src/oauth/chatgpt.rs:525:        let oauth = ChatGptOAuth::with_token_url(format!("http://127.0.0.1:{port}/oauth/token"));
crates/imp-llm/src/oauth/chatgpt.rs:546:        let oauth = ChatGptOAuth::with_token_url(format!("http://127.0.0.1:{port}/oauth/token"));
crates/imp-llm/src/oauth/chatgpt.rs:579:        let oauth = ChatGptOAuth::with_token_url(format!("http://127.0.0.1:{port}/oauth/token"));
docs/typescript-extension-bridge.md:76:  "version": "0.1.0",
docs/typescript-extension-bridge.md:166:  "version": "0.1.0",
docs/rebuild/imp-session-index-lifecycle-audit.md:69:- No index DB was found on this machine during `50.16.5.1` inspection.
docs/rebuild/imp-output-mode-contract.md:7:- planning notes in `.mana/50.16.1` document duplicated headless/RPC JSON encoders and the target split;
docs/rebuild/imp-output-mode-contract.md:8:- `.mana/50.17` captures the follow-on output-contract requirement;
crates/imp-llm/Cargo.toml:24:sha2 = "0.10"
crates/imp-llm/Cargo.toml:25:base64 = "0.22"
docs/mana-next-migration-test-plan.md:56:  100.1-child-task.md
docs/mana-next-migration-test-plan.md:57:  100.2-task-with-verify.md
docs/mana-next-migration-test-plan.md:58:  100.3-fact.md
docs/rebuild/imp-session-storage-search-recovery-audit.md:3:This audit resolves mana unit `50.16.5.1`: why `session_search` can report no indexed sessions even when raw session transcripts exist, and where an operator should look for recovery on this machine.
docs/rebuild/imp-workflow-feature-inventory.md:1:# imp Workflow Feature Inventory for 0.3
docs/rebuild/imp-workflow-feature-inventory.md:5:This inventory reconciles the current 0.3 direction with existing imp/mana/work/prototype surfaces. The working direction comes from `.imp/workflows`, especially:
docs/rebuild/imp-workflow-feature-inventory.md:14:The important direction change: imp-native workflows are the intended primary orchestration capability for imp 0.3. They may replace normal imp use of mana, work, and prototype once workflow parity exists. Older mana-first docs and tasks are historical context, not normative 0.3 product direction unless explicitly revived.
docs/rebuild/imp-workflow-feature-inventory.md:26:| Feature/surface | Current role | 0.3 disposition | Rationale | Removal/parity condition |
docs/rebuild/imp-workflow-feature-inventory.md:29:| imp-work / work tool | Durable tasks, epics, memory, decisions, context packs, runs, checks, handoff | Remove from default imp; keep archived/compatibility-only outside default runtime | Default imp 0.2.6 already removed imp-work from `imp-cli` dependency tree. Workflow artifacts should replace normal imp use of work/task state. | Safe to keep stripped from default. Any reintroduction must be explicit migration/import only. |
docs/rebuild/imp-workflow-feature-inventory.md:31:| mana integration | Optional mana command/tool/UI integration | Compatibility-only / optional adapter | 0.3 should not depend on mana for normal execution. mana may remain useful for old graphs or external experiments. | Keep behind `mana-ui` / `mana-tool`; default dependency tree must stay free of `mana-core` and `mana-cli`. |
docs/rebuild/imp-workflow-feature-inventory.md:32:| mana-first 365 child specs | Prior target architecture around mana harness | Defer/supersede for 0.3 | The active workflow artifacts contradict mana-first acceptance. Continuing those specs would create stale product direction. | Create a superseding workflow epic or rewrite 365 before doing more mana-harness spec work. |
docs/rebuild/imp-workflow-feature-inventory.md:36:| Context packs | Prepared context for tasks/workers/prototypes | Fold into workflows | Context belongs near the workflow step that needs it. Workflows should define required files, symbols, searches, freshness, and worker-specific bundles. | Blocked on workflow context schema/runtime support. Do not create a separate durable context-pack store for 0.3. |
docs/rebuild/imp-workflow-feature-inventory.md:38:| Bounded subagents | Runtime-local worker orchestration | Keep and implement as workflow execution primitive | Workflows need real workers for build/review/prototype/check steps. Subagents are runtime execution, not a separate durable work graph. | Required for 0.3 if workflows promise orchestration beyond serial shell steps. |
docs/rebuild/imp-workflow-feature-inventory.md:95:2. **Update README for 0.3 workflow direction**
docs/rebuild/imp-workflow-feature-inventory.md:117:   - Either supersede old mana-first children or rewrite the epic into workflow-first 0.3 planning.
docs/rebuild/imp-workflow-feature-inventory.md:122:## 0.3 release gate
docs/rebuild/imp-workflow-feature-inventory.md:124:Before bumping to 0.3, imp should have:
crates/imp-core/Cargo.toml:16:mana = { version = "0.3.2", package = "mana-cli", optional = true }
crates/imp-core/Cargo.toml:30:glob = "0.3"
crates/imp-core/Cargo.toml:31:pathdiff = "0.2"
crates/imp-core/Cargo.toml:63:rusqlite = { version = "0.39", features = ["bundled"] }
crates/imp-core/Cargo.toml:66:libc = "0.2"
docs/plans/pi-provider-oauth-parity.md:11:| Anthropic | OAuth browser/loopback callback plus refresh | `/Users/asher/pi-mono/packages/ai/src/utils/oauth/anthropic.ts`, `index.ts` | authorize `https://claude.ai/oauth/authorize`, token `https://platform.claude.com/v1/oauth/token`, loopback `127.0.0.1:53692/callback`, scopes include `org:create_api_key`, `user:profile`, `user:inference`, Claude Code/session/MCP/file upload scopes | Anthropic/Claude models in generated registry | OAuth subscription route; Node/Bun callback server only. |
crates/imp-core/src/guardrails.rs:505:            "[package]\nname='x'\nversion='0.1.0'\n",
crates/imp-core/src/typescript_extensions/bun_runner.rs:367:            version: "0.1.0".into(),
crates/imp-lua/Cargo.toml:22:libc = "0.2"
crates/imp-core/src/typescript_extensions/schema.rs:368:            version: "0.1.0".into(),
crates/imp-core/src/typescript_extensions/schema.rs:415:            "version": "0.1.0",
crates/imp-core/src/typescript_extensions/schema.rs:479:        assert_eq!(metadata.manifest_version.as_deref(), Some("0.1.0"));
docs/design/imp-work-mana-feature-parity.md:68:2. Finish `360.1`/core workflow helpers and commit cleanly.
crates/imp-core/examples/tool_ab_harness.rs:81:                cache_read_per_mtok: 0.3,
crates/imp-core/src/typescript_extensions/mod.rs:363:                "version": "0.1.0",
crates/imp-core/src/typescript_extensions/mod.rs:420:                    "version": "0.1.0",
crates/imp-core/src/typescript_extensions/mod.rs:499:                "version": "0.1.0",
crates/imp-core/src/typescript_extensions/mod.rs:528:        assert_eq!(metadata.manifest_version.as_deref(), Some("0.1.0"));
crates/imp-core/src/builder.rs:517:                    cache_read_per_mtok: 0.3,
docs/design/lua-programmatic-interactions.md:71:imp.ui.request({ kind = "input", title = "Version", placeholder = "0.1.3" })
crates/imp-cli/Cargo.toml:33:futures-util = "0.3"
crates/imp-core/src/retry.rs:212:        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
crates/imp-tui/Cargo.toml:33:unicode-width = "0.2"
crates/imp-core/src/agent/mod.rs:1092:                    cache_read_per_mtok: 0.3,
crates/imp-core/src/agent/mod.rs:5056:                    cache_read_per_mtok: 0.3,
crates/imp-core/src/agent/mod.rs:5644:                    cache_read_per_mtok: 0.3,
crates/imp-tui/src/app.rs:8393:                            temperature: Some(0.2),
crates/imp-tui/src/app.rs:9732:            output: 0.25,
crates/imp-tui/src/views/top_bar.rs:115:            cost: 0.12,
docs/dependency-audit.md:12:- `lru 0.12.5` (`RUSTSEC-2026-0002`) and `paste 1.0.15` (`RUSTSEC-2024-0436`) come from `ratatui 0.29.0`.
docs/dependency-audit.md:14:  - `cargo update -p lru --precise 0.16.3` is blocked by `ratatui`'s `lru = ^0.12` requirement.
docs/dependency-audit.md:15:  - `ratatui 0.30.0` upgrades `lru` but pulls a broad ~48-package update, so treat it as a deliberate UI dependency migration.
docs/dependency-audit.md:17:- `serde_yml 0.0.12` (`RUSTSEC-2025-0068`) and `libyml 0.0.5` (`RUSTSEC-2025-0067`) come from `mana-core 0.3.2`.
docs/dependency-audit.md:21:- `fxhash 0.2.1` (`RUSTSEC-2025-0057`) comes from `readability-rust 0.1.0 -> scraper 0.18.1 -> selectors 0.25.0`.
docs/dependency-audit.md:23:  - Forcing `scraper 0.24+` or `selectors 0.32+` is blocked by semver constraints.
docs/dependency-audit.md:28:- `cargo update -p scraper --precise 0.24.0 --dry-run` fails because `readability-rust` requires `scraper = ^0.18`.
docs/dependency-audit.md:29:- `cargo update -p selectors --precise 0.32.0 --dry-run` fails because `scraper 0.18.1` requires `selectors = ^0.25.0`.
docs/dependency-audit.md:30:- `cargo update -p lru --precise 0.16.3 --dry-run` fails because `ratatui 0.29.0` requires `lru = ^0.12.0`.
crates/imp-core/src/usage.rs:674:                cache_read: 0.1,
crates/imp-core/src/usage.rs:675:                cache_write: 0.2,
crates/imp-core/src/usage.rs:736:                    output: 0.12,
crates/imp-core/src/usage.rs:761:            cache_read: 0.3,
crates/imp-core/src/usage.rs:808:            cache_read: 0.3,
crates/imp-core/src/tools/web/read.rs:15:    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
crates/imp-core/src/tools/web/read.rs:388:        && (page.content_length as f64) < (page.raw_body_bytes as f64 * 0.1)
crates/imp-core/src/tools/web/read.rs:451:        && (page.content_length as f64) < (page.raw_body_bytes as f64 * 0.1)
crates/imp-core/src/tools/web/read.rs:552:            "http://127.0.0.1",
crates/imp-core/src/tools/web/read.rs:553:            "http://10.0.0.1",
crates/imp-core/src/tools/web/read.rs:581:            "http://127.0.0.1:8080/admin",
crates/imp-core/src/tools/scan/mod.rs:2013:            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
crates/imp-core/src/tools/scan/mod.rs:2131:            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",

## Potential stale promises
crates/imp-lua/README.md:35:Lua is the current stable shipped extension path for imp. TypeScript compatibility exists elsewhere in imp, but it is more limited and still evolving.
crates/imp-gui/README.md:5:This crate starts as a standalone shell for the imp workbench UI: project status, mana work navigation, execution timeline, inspector, diff preview, and terminal output panes. Runtime integration will be added incrementally after the layout and app boundary are stable.
crates/imp-core/README.md:5:It contains the agent loop, tool registry, session persistence, context assembly, hooks, policy/mode enforcement, mana integration, and the early Rust SDK surface used by hosts that want to embed imp.
crates/imp-core/README.md:14:- mana task execution support
crates/imp-cli/README.md:5:It builds the `imp` binary and wires together the terminal UI, CLI chat shell, one-shot prompt mode, auth/setup commands, secrets commands, mana task execution, import helpers, and RPC/headless entrypoints.
crates/imp-cli/README.md:15:- direct mana task execution via `imp run <unit-id>`
crates/imp-cli/README.md:36:The CLI is an active user-facing surface. Some headless/RPC-oriented paths are still evolving; the normal terminal UI, CLI chat, auth/secrets, and direct mana execution workflows are the primary supported surfaces.
README.md:3:Local terminal coding agent in Rust. imp runs through the TUI, one-shot prompts, or a JSONL RPC protocol. It uses structured tools, durable sessions, and file-backed workflows for planned, inspectable development work.
README.md:53:- preview Rust SDK
README.md:210:Current storage is local and file-backed. API-addressable workflows are planned.
README.md:248:| `/secrets` | credential manager |
README.md:403:- Rust SDK preview
README.md:405:Preview/planned:
README.md:408:- MCP planned
README.md:409:- `.imp/agents` planned
README.md:410:- ACP editor adapter scaffold
README.md:411:- hosted sync/team collaboration planned
README.md:412:- workflow API planned
README.md:416:- legacy `mana` integration is optional and compatibility-oriented
README.md:417:- TypeScript/Pi extension compatibility is experimental and not a shipped extension surface
README.md:423:- [ACP editor adapter](docs/acp.md)
docs/proposals/tool-review-2026-04.md:113:### `ask`, `web`, `mana`, `extend`, `memory`, `scan` — keep
docs/proposals/tool-review-2026-04.md:137:mana, memory, read, scan, session_search, web, write
docs/proposals/tool-review-2026-04.md:143:ask, bash, edit, extend, mana, memory, read, scan,
docs/proposals/guest-runtime-implementation-plan.md:11:- Do not treat JavaScript or TypeScript as Phase 1; they are optional future guests after the substrate is proven.
docs/proposals/guest-runtime-implementation-plan.md:77:## Phase 4 — Evolve `extend` into extension authoring/management
docs/proposals/guest-runtime-implementation-plan.md:79:Goal: keep `extend` as an authoring and management helper, not a script execution tool.
docs/proposals/guest-runtime-implementation-plan.md:111:- security review before enabling beyond local experimental profiles.
docs/proposals/guest-runtime-implementation-plan.md:129:3. Migrate Lua before adding JavaScript/TypeScript so current shipped users are not stranded.
docs/proposals/guest-runtime-implementation-plan.md:130:4. Keep `extend` focused on durable extension authoring/management.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:6:> runtime while preserving `mana run` as the durable parallel
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:7:> orchestrator, and preserving imp's native `mana` tool as the intended
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:20:imp native mana tool = first-class orchestration UX
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:21:mana run            = orchestration engine / durable parallel dispatch
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:30:separate durable orchestration substrate alongside mana.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:33:- imp's native `mana` tool remains the primary way a human or agent
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:35:- `mana run` remains responsible for selecting ready work, scheduling
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:39:  `mana run` dispatches for one unit at a time.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:47:`imp/crates/imp-core/src/tools/mana.rs` already provides:
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:48:- `mana(action="run")`
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:49:- `mana(action="run_state")`
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:50:- `mana(action="evaluate")`
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:51:- `mana(action="logs")`
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:60:`mana/crates/mana-cli/src/commands/run/mod.rs` already owns:
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:75:- `load_mana_unit()` walks upward to find `.mana/`
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:80:- shells out to `mana close`
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:89:Make `imp run` the canonical worker/runtime for **one** mana unit,
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:92:- **imp native mana tool** should be the first-class user-facing path.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:93:- **mana run** should remain the orchestrator.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:99:> We are strengthening it so the native mana tool and mana orchestrator
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:106:### Imp native mana tool owns
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:108:- interactive invocation of mana orchestration
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:123:- execution of one assigned mana unit
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:132:- a second durable orchestration substrate separate from mana
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:144:  -> native mana tool
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:145:    -> mana run orchestration
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:151:1. The native `mana` tool in imp should remain the best-feeling way to
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:178:This is what `mana run` should dispatch:
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:182:  --mana-dir /path/to/.mana \
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:189:- explicit mana root / unit scope
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:203:Instead it should consume a canonical mana execution bundle or canonical
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:204:mana-core loading path that provides:
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:218:- a new mana-core API returning an execution bundle, or
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:219:- a shared contract type introduced at the imp↔mana boundary.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:223:> `imp run` should load execution data through a canonical mana contract,
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:240:versionable and explicit enough for `mana run` to consume without
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:282:- `imp run` should not shell out to `mana close`
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:283:- `mana run` should remain responsible for verify batching,
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:301:Used by `mana run`.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:305:- explicit mana root
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:307:- no shelling out to `mana close`
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:327:Because imp's native `mana` tool is the intended first-class UX, it must
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:331:- the native mana tool is the orchestration surface
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:332:- mana is the durable run engine
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:341:native mana tool -> start/run/inspect orchestration
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:342:mana run         -> coordinate units in parallel
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:356:3. Replace `mana run` with direct imp parallel execution.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:357:4. Collapse the imp native mana tool and `imp run` into one undifferentiated layer.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:375:- Move from markdown scanning toward canonical mana execution loading.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:378:### Phase 4 — wire mana run onto the contract
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:379:- `mana run` continues parallel orchestration.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:384:- Polish imp native mana tool wording and state surfaces.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:394:1. A human inside imp experiences the native `mana` tool as the primary
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:396:2. `mana run` still owns durable orchestration and parallel dispatch.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:401:   native mana tool for orchestration UX.
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:409:- `28.1.3` — integrate `mana run` with the strengthened worker contract
docs/proposals/imp-run-worker-contract-and-mana-run-handoff.md:411:- `28.1.5` — make the native mana tool the clear first-class orchestration UX
docs/proposals/mana-wiki-schema-and-workflow.md:5:> Defines the synthesized knowledge layer that sits on top of mana's
docs/proposals/mana-wiki-schema-and-workflow.md:9:> Depends on: `.10.1` (imp memory architecture and mana ownership boundaries)
docs/proposals/mana-wiki-schema-and-workflow.md:30:mana: the wiki leverages mana's verification model. Strong claims should
docs/proposals/mana-wiki-schema-and-workflow.md:31:point to `mana fact` entries or cite concrete sources. The wiki is not a
docs/proposals/mana-wiki-schema-and-workflow.md:41:of markdown files at `.mana/wiki/` that imp agents read and write using
docs/proposals/mana-wiki-schema-and-workflow.md:42:standard file tools. No mana-core commands required.
docs/proposals/mana-wiki-schema-and-workflow.md:44:Phase 2 (future): If the pattern proves valuable, mana-core could add
docs/proposals/mana-wiki-schema-and-workflow.md:45:`mana wiki ingest`, `mana wiki query`, `mana wiki lint` as first-class
docs/proposals/mana-wiki-schema-and-workflow.md:51:mana's core model. Forcing wiki pages into the unit model would weaken
docs/proposals/mana-wiki-schema-and-workflow.md:59:.mana/wiki/
docs/proposals/mana-wiki-schema-and-workflow.md:104:  How mana dispatches units to agents, manages waves,
docs/proposals/mana-wiki-schema-and-workflow.md:132:  - `unit:` — mana unit ID (open or archived).
docs/proposals/mana-wiki-schema-and-workflow.md:133:  - `fact:` — mana fact ID.
docs/proposals/mana-wiki-schema-and-workflow.md:137:- `related_pages` — paths to other wiki pages (relative to `.mana/wiki/`).
docs/proposals/mana-wiki-schema-and-workflow.md:175:Atomic, shell-provable claims. These live in `mana fact`, **not** in
docs/proposals/mana-wiki-schema-and-workflow.md:179:- "Project uses serde_yml 0.0.12" → `mana fact` with `grep` verify.
docs/proposals/mana-wiki-schema-and-workflow.md:180:- "close.rs is the largest command module" → `mana fact` with `wc -l` verify.
docs/proposals/mana-wiki-schema-and-workflow.md:182:**Rule:** If a claim can be expressed as a `mana fact` with a verify
docs/proposals/mana-wiki-schema-and-workflow.md:203:- "MCP tools may diverge from CLI close behavior" → filed as open question.
docs/proposals/mana-wiki-schema-and-workflow.md:212:- When a Tier 3 claim gets verified, convert it to a `mana fact` and
docs/proposals/mana-wiki-schema-and-workflow.md:214:- When a `mana fact` fails re-verification, wiki pages citing it should
docs/proposals/mana-wiki-schema-and-workflow.md:229:- [Orchestration](systems/orchestration.md) — How mana dispatches units to agents.
docs/proposals/mana-wiki-schema-and-workflow.md:230:- [Session Memory](systems/session-memory.md) — How imp manages conversation history.
docs/proposals/mana-wiki-schema-and-workflow.md:239:- [Debugging a Stuck Run](playbooks/debugging-stuck-run.md) — Steps when mana run hangs.
docs/proposals/mana-wiki-schema-and-workflow.md:267:Parseable with `grep "^## \[" .mana/wiki/log.md | tail -5`.
docs/proposals/mana-wiki-schema-and-workflow.md:333:   `mana verify-facts` or gone past TTL.
docs/proposals/mana-wiki-schema-and-workflow.md:342:in `mana context` (no-ID) output.
docs/proposals/mana-wiki-schema-and-workflow.md:357:**The rule:** If it has a verify command, it is a `mana fact`. If it is
docs/proposals/mana-wiki-schema-and-workflow.md:358:active work, it is a `mana unit`. If it is maintained understanding,
docs/proposals/mana-wiki-schema-and-workflow.md:383:When citing a `mana fact`, include the fact title for human readability:
docs/proposals/mana-wiki-schema-and-workflow.md:394:- No numeric prefixes (unlike mana units). Wiki pages are navigated by
docs/proposals/mana-wiki-schema-and-workflow.md:404:`.mana/wiki/` should be **tracked in git** (unlike `index.yaml` which
docs/proposals/mana-wiki-schema-and-workflow.md:415:For an existing project with mana history:
docs/proposals/mana-wiki-schema-and-workflow.md:417:1. Create `.mana/wiki/index.md` and `.mana/wiki/log.md`.
docs/proposals/mana-wiki-schema-and-workflow.md:433:- The agent discovers pages by reading `.mana/wiki/index.md`.
docs/proposals/mana-wiki-schema-and-workflow.md:438:  (cwd, active files, mana task paths) and inject them into the system
docs/proposals/mana-wiki-schema-and-workflow.md:440:- Stale wiki pages (flagged by lint) could appear in `mana context`
docs/proposals/mana-wiki-schema-and-workflow.md:442:- A future `/wiki` slash command or `mana wiki query` could provide
docs/proposals/mana-wiki-schema-and-workflow.md:447:- The sidebar could distinguish wiki knowledge from raw mana status.
docs/proposals/mana-wiki-schema-and-workflow.md:453:The mana wiki is a `.mana/wiki/` directory of LLM-maintained markdown
docs/proposals/mana-wiki-schema-and-workflow.md:454:pages with YAML frontmatter. It sits on top of mana's existing units and
docs/proposals/guest-runtime-extension-substrate.md:192:- replace Lua with TypeScript
docs/proposals/guest-runtime-extension-substrate.md:221:A guest runtime is a host-managed execution environment that can:
docs/proposals/guest-runtime-extension-substrate.md:425:Those stay in the Rust runtime / mana worker boundary.
docs/proposals/guest-runtime-extension-substrate.md:536:- in future, treat it as authoring/management UX, not as the architecture boundary
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:5:> Defines how imp should surface mana work state and synthesized
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:17:1. **Work state** — what mana jobs exist, what is running, what just
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:33:| Sidebar mana formatting | `views/sidebar.rs` | Expanded mana tool call details (create params, status results, run config) |
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:34:| Tool call summary | `views/tools.rs` | Compact one-line mana action summaries (`format_mana_args`) |
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:38:### What is planned but not built
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:42:| `27.2` | Compact mana status/progress widget | Open, blocked on `27.1` + `30` |
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:43:| `27.3` | Non-blocking mana follow-ups and message delivery | In progress, 1 abandoned attempt |
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:51:A single compact status row displayed between turns when mana work is
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:56:- After imp creates a mana unit during conversation.
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:57:- While background mana work is running.
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:58:- After a mana run completes (success or failure).
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:62:┌─ mana ────────────────────────────────────────┐
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:69:(no mana status row shown)
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:73:- Updates after each mana tool call returns.
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:75:- Clicking (mouse) or a keybind could expand to full `mana status` output.
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:84:When a background mana run completes, surface a non-intrusive
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:88:- After a mana unit dispatched in the background closes (success or failure).
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:93:┌─ mana ──────────────────────────────────────┐
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:94:│ ✓ .10.3 closed: Strengthen mana-first prompt │
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:138:The existing sidebar already formats mana tool output. Enrich it with:
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:140:- **After `mana create`:** Show the created unit's title, ID, and verify
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:142:- **After `mana status`/`mana list`:** Show a structured summary instead
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:144:- **After `mana run`:** Show which units were dispatched and their
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:148:`format_mana_output()` in `views/sidebar.rs`.
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:154:1. **No persistent mana dashboard panel.** imp's TUI is a conversation
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:155:   interface, not a project management tool. Mana state surfaces between
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:162:   are managed by mana, not imp's TUI. Completion notifications are
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:165:4. **No automatic mana actions.** The TUI shows state; it does not
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:179:- Richer sidebar formatting for mana tool results (Surface 4).
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:183:- `crates/imp-core/src/tools/mana.rs` — emit status metadata in tool output.
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:185:- `crates/imp-tui/src/views/sidebar.rs` — enrich `format_mana_output()`.
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:190:**Requires:** Non-blocking mana follow-up infrastructure.
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:232:1. **Status row** between turns showing active mana work (extends `27.2`).
docs/proposals/inline-mana-state-and-knowledge-surfaces.md:236:Plus sidebar enrichment for better mana tool output formatting (no new
docs/proposals/mana-aware-runtime-context-read-path.md:6:> mana-derived durable context (facts, wiki pages) without collapsing
docs/proposals/mana-aware-runtime-context-read-path.md:32:`builder.rs` always passes an empty slice. No mana facts are loaded at
docs/proposals/mana-aware-runtime-context-read-path.md:37:1. `load_mana_unit()` reads the unit markdown file from `.mana/`.
docs/proposals/mana-aware-runtime-context-read-path.md:53:Same as prompt assembly above. No mana-derived context is loaded at
docs/proposals/mana-aware-runtime-context-read-path.md:54:session start beyond what the agent discovers by using the `mana` tool
docs/proposals/mana-aware-runtime-context-read-path.md:71:**Implication for mana context:** Mana facts loaded at session start are
docs/proposals/mana-aware-runtime-context-read-path.md:76:However, mana **work status** (unit progress, new child jobs, completion
docs/proposals/mana-aware-runtime-context-read-path.md:101:add a "re-ground" operation that re-loads mana context after compaction.
docs/proposals/mana-aware-runtime-context-read-path.md:117:| Work status updates | During session | Tool results from mana tool | Per-turn |
docs/proposals/mana-aware-runtime-context-read-path.md:123:In `builder.rs`, after discovering `agents_md` and `skills`, load mana
docs/proposals/mana-aware-runtime-context-read-path.md:124:facts from `.mana/`:
docs/proposals/mana-aware-runtime-context-read-path.md:128:let facts = if self.cwd.join(".mana").exists() {
docs/proposals/mana-aware-runtime-context-read-path.md:129:    load_mana_facts(&self.cwd.join(".mana"))
docs/proposals/mana-aware-runtime-context-read-path.md:140:If `.mana/wiki/index.md` exists, read it and inject a compact summary
docs/proposals/mana-aware-runtime-context-read-path.md:147:- systems/orchestration.md — How mana dispatches units to agents.
docs/proposals/mana-aware-runtime-context-read-path.md:149:[12 pages total — see .mana/wiki/index.md]
docs/proposals/mana-aware-runtime-context-read-path.md:154:**Phase 1 — add mana memory context:**
docs/proposals/mana-aware-runtime-context-read-path.md:156:If `mana context` (no-ID) output is available, inject a compact version
docs/proposals/mana-aware-runtime-context-read-path.md:159:is bounded by mana's own truncation.
docs/proposals/mana-aware-runtime-context-read-path.md:168:2. If `.mana/wiki/index.md` exists, scan it for pages whose tags or
docs/proposals/mana-aware-runtime-context-read-path.md:190:2. **Do not refresh mana context mid-session.** The frozen snapshot
docs/proposals/mana-aware-runtime-context-read-path.md:198:4. **Do not make wiki pages mandatory.** If `.mana/wiki/` does not
docs/proposals/mana-aware-runtime-context-read-path.md:201:5. **Do not duplicate mana context across layers.** If a fact is in
docs/proposals/mana-aware-runtime-context-read-path.md:211:| Load mana facts at session start | `crates/imp-core/src/builder.rs` | Small — read `.mana/` facts, convert to `Fact` structs |
docs/proposals/mana-aware-runtime-context-read-path.md:214:| Load wiki index in builder | `crates/imp-core/src/builder.rs` | Small — read and parse `.mana/wiki/index.md` |
docs/proposals/mana-aware-runtime-context-read-path.md:225:  mana facts and wiki index to catch changes from other agents.
docs/proposals/mana-aware-runtime-context-read-path.md:230:- **Mana context streaming:** Surface mana memory context updates as
docs/proposals/mana-aware-runtime-context-read-path.md:239:1. **Populate Layer 4** — load mana facts into `builder.rs` (filling the
docs/proposals/mana-aware-runtime-context-read-path.md:248:gracefully when no mana facts or wiki exist.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:21:| 3. Mana work memory | Project-durable | `.mana/` unit + fact files | Agent (via mana tool), orchestrator, workers | Prompt assembly, headless dispatch, `mana context` |
docs/proposals/imp-memory-architecture-and-mana-boundary.md:22:| 4. Synthesized project knowledge (wiki layer) | Project-durable, compounding | `.mana/wiki/` markdown pages (future) | Agent (maintenance operations) | Prompt assembly, on-demand query, human browsing |
docs/proposals/imp-memory-architecture-and-mana-boundary.md:101:- Project-specific architecture knowledge (use mana facts or wiki).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:102:- Implementation plans or work status (use mana units).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:104:- Anything that requires verification (use `mana fact`).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:109:the same project, it belongs in mana.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:119:**Where it lives.** `.mana/` directory in the project root — YAML
docs/proposals/imp-memory-architecture-and-mana-boundary.md:124:- `mana` CLI and the imp-native `mana` tool (`crates/imp-core/src/tools/mana.rs`)
docs/proposals/imp-memory-architecture-and-mana-boundary.md:125:- `crates/imp-core/src/system_prompt.rs` — Layer 4 (mana facts), Layer 5 (task context)
docs/proposals/imp-memory-architecture-and-mana-boundary.md:130:- Agents write via the `mana` tool (create, update, close, fact_create, etc.).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:131:- Humans can edit `.mana/` files directly.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:132:- Workers record progress via `mana update --note`.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:136:- `mana context <id>` assembles a complete briefing for a specific unit.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:137:- `mana context` (no ID) outputs project-wide memory context (stale facts, claimed units, recent work).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:138:- `mana recall` searches units by keyword.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:144:- Verified project facts with shell-checkable proof (`mana fact`).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:156:- `builder.rs` passes `facts: &[]` — mana facts are not yet loaded into the system prompt at session start.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:157:- `mana context` (no ID) exists in mana CLI but is not yet wired into imp's session-start flow.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:160:**Anti-goal:** Do not use mana units as a freeform knowledge wiki. Units
docs/proposals/imp-memory-architecture-and-mana-boundary.md:170:markdown pages that sits between raw mana artifacts and future questions.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:179:**Where it will live.** `.mana/wiki/` — a directory of markdown files
docs/proposals/imp-memory-architecture-and-mana-boundary.md:180:with YAML frontmatter, managed by the agent, readable by humans and
docs/proposals/imp-memory-architecture-and-mana-boundary.md:196:- The wiki index serves as the navigation entry point (like `mana recall` but for synthesized knowledge).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:199:1. **Verified facts** — atomic claims with shell proof. These live in `mana fact`, not wiki pages. Wiki pages may cite them.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:211:- Raw work status (that is mana units).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:214:- Unverified claims presented as facts (use the hypothesis tier or `mana fact`).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:217:claim should either be a verified `mana fact` or cite specific sources
docs/proposals/imp-memory-architecture-and-mana-boundary.md:229:  implementation structure, externalize into mana units/facts during the
docs/proposals/imp-memory-architecture-and-mana-boundary.md:231:- When a unit closes, the completion record lives in mana (archived unit).
docs/proposals/imp-memory-architecture-and-mana-boundary.md:242:- The wiki cites mana artifacts; it does not replace them.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:280:- Wire mana facts into `builder.rs` → `facts_layer()` at session start.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:281:- Wire `mana context` (no-ID) output into session-start context for interactive sessions.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:290:- Create `.mana/wiki/` structure.
docs/proposals/imp-memory-architecture-and-mana-boundary.md:305:boundaries. Project knowledge flows outward from sessions into mana and
docs/eval-candidates.md:146:    "kind": "mana-unit",
docs/eval-candidates.md:148:    "path": ".mana"
docs/eval-candidates.md:210:- workflow id / mana unit id
docs/proposals/script-tool-boundaries-and-policy.md:21:- distinct from **`extend`**, which is an authoring/management surface
docs/proposals/script-tool-boundaries-and-policy.md:30:extend tool                     -> authoring/management helper
docs/proposals/script-tool-boundaries-and-policy.md:83:This makes it an authoring and management helper.
docs/proposals/script-tool-boundaries-and-policy.md:186:`extend` should remain an authoring/management helper.
docs/proposals/script-tool-boundaries-and-policy.md:468:- keep `extend` positioned as authoring/management
docs/run-evidence.md:3:Status: nightly slice for mana epic 432
docs/run-evidence.md:43:The schema is intentionally compact for nightly. Workflow-branch work can extend it with richer policy, verification, diff, and mana references.
docs/trust-labels-and-provenance.md:96:Tool outputs from read/search/bash/git/mana/web/etc. Trust depends on the tool,
docs/trust-labels-and-provenance.md:121:Durable mana ledger records. They are structured and reviewable, but still need
docs/trust-labels-and-provenance.md:174:   memory persistence, mana ledger writes, network/secrets/destructive access, or
docs/trust-labels-and-provenance.md:192:- mana record -> mana source type plus unit id
docs/trust-labels-and-provenance.md:218:- `mana show` -> mana ledger provenance
docs/trust-labels-and-provenance.md:226:Before writing durable memory, mana facts/notes/decisions, eval candidates, or
docs/trust-labels-and-provenance.md:244:[context id=ctx_22 source=mana-fact trust=mana-ledger unit=394.7 verified=true]
docs/trust-labels-and-provenance.md:274:- mana fact/note/decision writes as adopted truth
docs/trust-labels-and-provenance.md:324:mana: 394.7       mana-ledger
docs/trust-labels-and-provenance.md:342:2. Attach provenance during context assembly for user, workspace, web, mana,
docs/trust-labels-and-provenance.md:347:6. Gate durable memory/mana writes by provenance.
docs/trust-labels-and-provenance.md:373:- mana prompt-context provenance for facts and project memory status
docs/trust-labels-and-provenance.md:435:Future prompt context can add labels for web, mana, verifier, generated summaries,
docs/trust-labels-and-provenance.md:461:## Durable memory and mana writes
docs/trust-labels-and-provenance.md:532:  mana facts, autonomy escalation, or dangerous grants
docs/trust-labels-and-provenance.md:549:- `mana show` should produce mana-ledger provenance
docs/trust-labels-and-provenance.md:560:Lua and future TypeScript extensions are not trust boundaries. Extension manifests
docs/trust-labels-and-provenance.md:564:Future TypeScript extension manifests should declare:
docs/trust-labels-and-provenance.md:585:- mana write gating is specified but not fully enforced everywhere yet
docs/trust-labels-and-provenance.md:607:- Will this content be written to memory/mana/eval artifacts?
docs/trust-labels-and-provenance.md:640:- mana prompt-context provenance for facts and project memory status
docs/trust-labels-and-provenance.md:702:Future prompt context can add labels for web, mana, verifier, generated summaries,
docs/trust-labels-and-provenance.md:728:## Durable memory and mana writes
docs/trust-labels-and-provenance.md:799:  mana facts, autonomy escalation, or dangerous grants
docs/trust-labels-and-provenance.md:816:- `mana show` should produce mana-ledger provenance
docs/trust-labels-and-provenance.md:827:Lua and future TypeScript extensions are not trust boundaries. Extension manifests
docs/trust-labels-and-provenance.md:831:Future TypeScript extension manifests should declare:
docs/trust-labels-and-provenance.md:852:- mana write gating is specified but not fully enforced everywhere yet
docs/trust-labels-and-provenance.md:874:- Will this content be written to memory/mana/eval artifacts?
docs/reference-monitor-policy.md:23:6. Bash calls are checked for mana-equivalent commands with
docs/reference-monitor-policy.md:24:   `mana_bash_equivalent_hint` and can be blocked with a native mana-tool hint.
docs/reference-monitor-policy.md:25:7. Mana calls are checked with `evaluate_mana_policy`, which applies
docs/reference-monitor-policy.md:26:   `AgentMode::allows_mana_action` and records mana action class details.
docs/reference-monitor-policy.md:52:- `Full`: all tools and mana actions
docs/reference-monitor-policy.md:53:- `Worker`: implementation tools and limited progress-checkpoint mana actions
docs/reference-monitor-policy.md:54:- `Orchestrator`: mana orchestration with no direct file write tool
docs/reference-monitor-policy.md:56:- `Reviewer`: read-only inspection; no mana actions
docs/reference-monitor-policy.md:57:- `Auditor`: read/report-oriented code and mana inspection
docs/reference-monitor-policy.md:59:`AgentMode::allows_tool` and `AgentMode::allows_mana_action` are currently called
docs/reference-monitor-policy.md:71:`agent/mana_loop.rs` classifies mana actions (`inspect`, `lifecycle`,
docs/reference-monitor-policy.md:79:`mana_bash_equivalent_hint` blocks shell commands that should use the native
docs/reference-monitor-policy.md:80:mana tool. This is a policy check, not a bash implementation detail. The monitor
docs/reference-monitor-policy.md:118:       mana action policy
docs/reference-monitor-policy.md:147:- mana action and action class for mana calls
docs/reference-monitor-policy.md:162:(`bash`, `git`, `mana`), and schema/path extraction helpers. Later extension work
docs/reference-monitor-policy.md:235:- keep mana bash-equivalent blocking and mana action class details
docs/reference-monitor-policy.md:246:   `AgentMode`, `RunPolicy`, mana policy, bash-equivalent, repeated-call, schema,
docs/reference-monitor-policy.md:254:8. Document tool-author requirements for core, Lua, and future TypeScript
docs/reference-monitor-policy.md:270:  `edit`, `multi_edit`, `bash`, `git`, `worktree`, `mana`, `web`, `ask`,
docs/reference-monitor-policy.md:273:  mana action, and network host.
docs/reference-monitor-policy.md:277:- Adapter records for hook blocking, mana policy decisions, bash-equivalent
docs/reference-monitor-policy.md:321:- `action` for mana actions
docs/reference-monitor-policy.md:335:Future TypeScript extension manifests from 394.10 should include at least:
docs/reference-monitor-policy.md:355:- `mana_policy_allowed`
docs/reference-monitor-policy.md:356:- `mana_policy_blocked`
docs/reference-monitor-policy.md:399:- Hook, mana, bash-equivalent, repeated-call, schema, and guardrail outcomes have
docs/reference-monitor-policy.md:407:- Manifest-driven TypeScript extension metadata belongs to 394.10.
docs/reference-monitor-policy.md:426:  `edit`, `multi_edit`, `bash`, `git`, `worktree`, `mana`, `web`, `ask`,
docs/reference-monitor-policy.md:429:  mana action, and network host.
docs/reference-monitor-policy.md:433:- Adapter records for hook blocking, mana policy decisions, bash-equivalent
docs/reference-monitor-policy.md:477:- `action` for mana actions
docs/reference-monitor-policy.md:491:Future TypeScript extension manifests from 394.10 should include at least:
docs/reference-monitor-policy.md:511:- `mana_policy_allowed`
docs/reference-monitor-policy.md:512:- `mana_policy_blocked`
docs/reference-monitor-policy.md:555:- Hook, mana, bash-equivalent, repeated-call, schema, and guardrail outcomes have
docs/reference-monitor-policy.md:563:- Manifest-driven TypeScript extension metadata belongs to 394.10.
docs/mana-next-migration-test-plan.md:1:# mana-next Compatibility and Migration Test Plan
docs/mana-next-migration-test-plan.md:4:Parent: mana `394.3` / child `394.3.9`
docs/mana-next-migration-test-plan.md:8:Before changing mana internals, we need a compatibility test plan that protects current mana behavior while adding workflow-ledger views, sidecars, evidence refs, and imp adapters.
docs/mana-next-migration-test-plan.md:12:Use temporary mana roots whenever possible:
docs/mana-next-migration-test-plan.md:15:/tmp/imp-mana-test/.mana
docs/mana-next-migration-test-plan.md:18:Avoid mutating the developer's real `~/.mana` in automated tests.
docs/mana-next-migration-test-plan.md:22:- clean empty mana root
docs/mana-next-migration-test-plan.md:23:- existing mana root with old epic/task/fact/decision files
docs/mana-next-migration-test-plan.md:24:- mana root with new `ledger/` sidecars
docs/mana-next-migration-test-plan.md:33:mana template kind=task
docs/mana-next-migration-test-plan.md:34:mana list --count 1
docs/mana-next-migration-test-plan.md:35:mana show <id>
docs/mana-next-migration-test-plan.md:36:mana create --kind task --title "..."
docs/mana-next-migration-test-plan.md:37:mana update <id> --notes "..."
docs/mana-next-migration-test-plan.md:38:mana verify <id>
docs/mana-next-migration-test-plan.md:39:mana close <id>
docs/mana-next-migration-test-plan.md:40:mana notes_append <id> --notes "..."
docs/mana-next-migration-test-plan.md:41:mana decision_add <id> --title "..."
docs/mana-next-migration-test-plan.md:42:mana decision_resolve <id> --resolve_decisions "..."
docs/mana-next-migration-test-plan.md:43:mana dep_add --from-id <id> --dep-id <dep>
docs/mana-next-migration-test-plan.md:44:mana dep_remove --from-id <id> --dep-id <dep>
docs/mana-next-migration-test-plan.md:47:If exact CLI syntax differs, use native mana tool equivalents in integration tests.
docs/mana-next-migration-test-plan.md:54:fixtures/mana-old/
docs/mana-next-migration-test-plan.md:74:fixtures/mana-next/
docs/mana-next-migration-test-plan.md:85:- existing mana list/show still sees `200-workflow.md`
docs/mana-next-migration-test-plan.md:89:- child run refs do not break old mana commands
docs/mana-next-migration-test-plan.md:109:- artifact content is not inlined into mana record
docs/mana-next-migration-test-plan.md:146:Input: unresolved current mana decision.
docs/mana-next-migration-test-plan.md:165:- no raw trace content written to mana markdown
docs/mana-next-migration-test-plan.md:198:- current mana status maps consistently
docs/mana-next-migration-test-plan.md:206:Given old mana files only:
docs/mana-next-migration-test-plan.md:214:Given old mana file, after imp writes evidence ref:
docs/mana-next-migration-test-plan.md:226:- Delete `~/.mana/ledger/` sidecars: old mana list/show still works.
docs/mana-next-migration-test-plan.md:227:- Delete `.imp/runs/<run-id>/` artifact: mana shows broken evidence ref gracefully.
docs/mana-next-migration-test-plan.md:228:- Disable mana-next adapter: current mana commands remain usable.
docs/mana-next-migration-test-plan.md:239:## CI gates for mana-next implementation
docs/mana-next-migration-test-plan.md:244:cargo test -p imp-core mana_workflow_ledger
docs/mana-next-migration-test-plan.md:245:cargo test -p imp-core mana_workflow_ledger_adapter
docs/mana-next-migration-test-plan.md:252:cargo test -p imp-core mana_next_fixtures
docs/mana-next-migration-test-plan.md:255:Manual smoke with native mana tool:
docs/mana-next-migration-test-plan.md:258:mana template kind=task
docs/mana-next-migration-test-plan.md:259:mana list --count 1
docs/mana-next-migration-test-plan.md:260:mana show <known-id>
docs/mana-next-migration-test-plan.md:265:The mana-next implementation is migration-ready only when:
docs/mana-next-migration-test-plan.md:272:- evidence/log content is not inlined into mana records
docs/runtime-event-state-api.md:6:and future GUI consumers. The contract lives in `imp-core` and keeps semantic
docs/runtime-event-state-api.md:17:layer for shared frontend/RPC/GUI state, not a replacement for the agent loop.
docs/runtime-event-state-api.md:71:- `mana_updated`
docs/runtime-event-state-api.md:116:    pub mana_refs: Vec<RuntimeManaRef>,
docs/runtime-event-state-api.md:133:- mana refs
docs/runtime-event-state-api.md:134:- compact status items useful for CLI/TUI/GUI rendering
docs/runtime-event-state-api.md:154:mana refs, warnings/errors, timing/recovery status, and unknown future events.
docs/runtime-event-state-api.md:168:| tool output previews | core semantic preview + TUI rendering | `RuntimeToolCall.output_preview` |
docs/runtime-event-state-api.md:182:## GUI guidance
docs/runtime-event-state-api.md:185:Recommended GUI adapter shape:
docs/runtime-event-state-api.md:199:The GUI can render representative snapshots in tests without launching a live
docs/plans/pi-provider-oauth-parity.md:11:| Anthropic | OAuth browser/loopback callback plus refresh | `/Users/asher/pi-mono/packages/ai/src/utils/oauth/anthropic.ts`, `index.ts` | authorize `https://claude.ai/oauth/authorize`, token `https://platform.claude.com/v1/oauth/token`, loopback `127.0.0.1:53692/callback`, scopes include `org:create_api_key`, `user:profile`, `user:inference`, Claude Code/session/MCP/file upload scopes | Anthropic/Claude models in generated registry | OAuth subscription route; Node/Bun callback server only. |
docs/imp-next-workflow-runtime.md:4:Audience: imp maintainers, mana maintainers, future TUI/GUI/runtime implementers
docs/imp-next-workflow-runtime.md:10:The default user experience remains the imp TUI. CLI entrypoints remain important for scripting, headless automation, and CI. A future GUI should not require a second runtime; it should consume the same event stream and state snapshots as the TUI.
docs/imp-next-workflow-runtime.md:15:TUI / CLI / future GUI
docs/imp-next-workflow-runtime.md:22:        -> mana workflow ledger
docs/imp-next-workflow-runtime.md:25:Rust remains the authority boundary for runtime execution, policy, tool scheduling, tracing, evidence, secrets, worktree/sandbox control, and durable state writes. TypeScript support is a future extension path through a host-controlled manifest + subprocess/JSON-RPC-style boundary, not a replacement for the Rust runtime.
docs/imp-next-workflow-runtime.md:32:4. Make mana the streamlined durable workflow/evidence ledger, not a noisy project-management UI.
docs/imp-next-workflow-runtime.md:35:7. Prepare a future GUI by exposing stable runtime events and state snapshots.
docs/imp-next-workflow-runtime.md:36:8. Add TypeScript extension support through manifests and Rust-enforced capabilities.
docs/imp-next-workflow-runtime.md:37:9. Delay multi-agent teams until the single-workflow runtime is trustworthy.
docs/imp-next-workflow-runtime.md:41:- Do not start by building OMO/OMX-style multi-agent teams.
docs/imp-next-workflow-runtime.md:43:- Do not replace Lua immediately or describe TypeScript support as fully shipped before it is.
docs/imp-next-workflow-runtime.md:45:- Do not turn mana into Jira. Mana should be a workflow ledger and evidence index.
docs/imp-next-workflow-runtime.md:68:- **mana** = platform / durable graph layer
docs/imp-next-workflow-runtime.md:69:- **imp** = agent + default human-facing environment on mana
docs/imp-next-workflow-runtime.md:90:│ TUI / CLI / future GUI                        │
docs/imp-next-workflow-runtime.md:156:- Mana-backed or task-backed runs can attach a `mana_unit_ref`, but no mana ledger writes are implemented by this slice.
docs/imp-next-workflow-runtime.md:164:- mana workflow-ledger writes
docs/imp-next-workflow-runtime.md:166:- TypeScript extension support
docs/imp-next-workflow-runtime.md:189:The TUI should not force users to manually manage workflow IDs for simple requests.
docs/imp-next-workflow-runtime.md:210:### Future GUI
docs/imp-next-workflow-runtime.md:212:A GUI should consume the same runtime event stream and state snapshots as the TUI. Avoid TUI-only state models in `imp-core`.
docs/imp-next-workflow-runtime.md:214:The GUI needs these stable surfaces:
docs/imp-next-workflow-runtime.md:227:A workflow contract should be created implicitly for normal TUI runs and explicitly for mana/task/CI runs.
docs/imp-next-workflow-runtime.md:246:  mana_unit_ref?
docs/imp-next-workflow-runtime.md:311:Current mana epics/tasks/facts/decisions can remain compatible, but the new runtime should write more structured workflow records.
docs/imp-next-workflow-runtime.md:325:Mana should not store raw transcript spam. Raw event traces belong in run artifacts; mana stores pointers and durable summaries.
docs/imp-next-workflow-runtime.md:378:- planned approach or reason no plan was needed
docs/imp-next-workflow-runtime.md:422:- hooks/mana policy
docs/imp-next-workflow-runtime.md:469:Durable memory/mana writes derived from low-trust content should be scoped to the workflow or require review.
docs/imp-next-workflow-runtime.md:521:## TypeScript extension boundary
docs/imp-next-workflow-runtime.md:523:TypeScript should be a future extension path, not a second runtime authority.
docs/imp-next-workflow-runtime.md:593:Do not start with teams. After single-workflow correctness is established, add child workflows inspired by OMO.
docs/imp-next-workflow-runtime.md:618:The TUI and future GUI need stable events.
docs/imp-next-workflow-runtime.md:627:tool.planned
docs/imp-next-workflow-runtime.md:638:mana.updated
docs/imp-next-workflow-runtime.md:653:- mana unit refs
docs/imp-next-workflow-runtime.md:669:- Add mana workflow/evidence ledger adapter.
docs/imp-next-workflow-runtime.md:684:- leaves TypeScript extension manifests to 394.10
docs/imp-next-workflow-runtime.md:696:- Add manifest-driven TypeScript tool bridge behind policy.
docs/imp-next-workflow-runtime.md:702:- Only then consider richer team orchestration.
docs/imp-next-workflow-runtime.md:707:2. How should mana workflow records map to existing epic/task/fact files without breaking old mana usage?
docs/imp-next-workflow-runtime.md:710:5. Should TypeScript extensions use Bun, Node, or a protocol that allows either?
docs/imp-next-workflow-runtime.md:722:- Future GUI can be built without reinventing runtime state.
docs/tui-workflow-wireframes.md:4:Scope: improve the current TUI first; keep the design GUI-ready through shared runtime state  
docs/tui-workflow-wireframes.md:5:Related epic: `394 Evolve imp into workflow-first agent runtime with mana ledger and extension support`
docs/tui-workflow-wireframes.md:25:  - It has bottom-left labels: mana scope, mana run, build loop/loop state.
docs/tui-workflow-wireframes.md:33:  - It can show mana run detail or thinking when no tool is selected.
docs/tui-workflow-wireframes.md:62:The current imp TUI should remain conversation-first. Workflow-first features should make the existing TUI more legible, not turn it into a project-management dashboard.
docs/tui-workflow-wireframes.md:64:A future GUI may make sense for richer evidence browsing, diff review, worktree management, and child workflow supervision. But the GUI should consume the same runtime state as the TUI:
docs/tui-workflow-wireframes.md:69:  -> future GUI presentation
docs/tui-workflow-wireframes.md:82:8. **Closeout should be satisfying.** Final summaries should make evidence, verification, diff, and mana status easy to inspect.
docs/tui-workflow-wireframes.md:99:│                                              │ - mana run detail            ││
docs/tui-workflow-wireframes.md:104:│ bottom-left: mana/build/loop labels                                           │
docs/tui-workflow-wireframes.md:210:Mana detail       current mana run detail pattern
docs/tui-workflow-wireframes.md:401:│ e open evidence · d diff · m mana · gpt-5.1-codex · main                    │
docs/tui-workflow-wireframes.md:442:Current `SidebarView` can show mana run detail when no tool is selected. Build on that.
docs/tui-workflow-wireframes.md:466:Do not put mana in the main chat unless the user asked to inspect/update it.
docs/tui-workflow-wireframes.md:540:Do not invent a team dashboard in the first TUI pass. Use sidebar/detail.
docs/tui-workflow-wireframes.md:575:# Part B — future GUI, grounded in current state model
docs/tui-workflow-wireframes.md:577:A future GUI can be richer, but it should still reflect the current TUI concepts:
docs/tui-workflow-wireframes.md:585:## 19. GUI shell
docs/tui-workflow-wireframes.md:609:## 20. GUI detail tabs
docs/tui-workflow-wireframes.md:620:## 21. GUI approval modal maps to AskBar semantics
docs/tui-workflow-wireframes.md:634:The GUI can render this as a modal, but it should come from the same ask/approval state as the TUI.
docs/tui-workflow-wireframes.md:636:## 22. GUI worktree/diff view
docs/tui-workflow-wireframes.md:638:This is where GUI becomes more valuable than TUI.
docs/tui-workflow-wireframes.md:653:## 23. GUI evidence view
docs/tui-workflow-wireframes.md:676:## 24. GUI should remain optional
docs/tui-workflow-wireframes.md:683:- mana ledger
docs/tui-workflow-wireframes.md:685:The GUI is richer inspection/control, not the source of truth.
docs/tui-workflow-wireframes.md:702:- Add mana detail using current mana run detail pattern.
docs/tui-workflow-wireframes.md:718:## 29. Phase 5: GUI prototype over shared state
docs/tui-workflow-wireframes.md:728:No GUI-only workflow state.
docs/tui-workflow-wireframes.md:738:7. Should GUI be a local desktop app, browser UI served by imp, or embedded app shell?
docs/release-promotions/current.md:30:| `eb3f46f` Use published mana crates for release build | release-only | Expected stable packaging behavior if release should build against published mana crates. |
docs/release-promotions/current.md:44:| `0184de6..31e1a04` workflow sprint branch | defer | Demonstrated performance/UX gain, mana worker contract smoke, TUI event-loop smoke, and full release gate. |
docs/acp.md:1:# ACP editor adapter
docs/acp.md:3:imp has an early Agent Client Protocol (ACP) stdio adapter behind:
docs/acp.md:9:ACP is a JSON-RPC protocol for editor/agent integration. The adapter is being built as a sibling to imp's existing `--mode rpc` JSONL worker protocol; the two protocols are intentionally not wire-compatible.
docs/acp.md:19:- `initialize` handshake for ACP protocol version 1.
docs/acp.md:22:- durable imp session creation for ACP sessions.
docs/acp.md:25:- initial imp event to ACP `session/update` mapping helpers.
docs/acp.md:29:- live `Agent::run` turn execution from ACP prompts.
docs/acp.md:32:- full policy-denial-to-ACP UX.
docs/acp.md:34:- client-supplied MCP server connections.
docs/acp.md:36:- ACP registry metadata.
docs/acp.md:40:`imp acp` uses ACP's stdio transport:
docs/acp.md:44:- stdout must contain only ACP JSON-RPC messages.
docs/acp.md:63:Editors that support custom ACP agents generally need a command and args. Use:
docs/acp.md:83:- If the editor reports invalid JSON, make sure nothing writes logs to stdout in ACP mode.
docs/acp.md:85:- If MCP server configuration fails, remove client-supplied MCP servers for now; imp does not yet advertise MCP capabilities.
docs/architecture.md:28:- preview Rust SDK
docs/architecture.md:55:- MCP
docs/architecture.md:57:- ACP/editor adapters
docs/architecture.md:58:- hosted sync/team collaboration
docs/mana-next-runtime-event-mapping.md:1:# mana-next Runtime Event Mapping
docs/mana-next-runtime-event-mapping.md:4:Parent: mana `394.3` / child `394.3.6`
docs/mana-next-runtime-event-mapping.md:8:This document defines which imp runtime events should create durable mana-next workflow ledger updates.
docs/mana-next-runtime-event-mapping.md:12:- **mana stores durable summaries and artifact refs**
docs/mana-next-runtime-event-mapping.md:16:Do not write every event to mana. Most runtime events belong only in `trace.jsonl`.
docs/mana-next-runtime-event-mapping.md:21:<thead><tr><th>Runtime event</th><th>mana update</th><th>Write policy</th><th>Notes</th></tr></thead>
docs/mana-next-runtime-event-mapping.md:23:<tr><td><code>workflow.started</code></td><td>Create/update WorkflowRecord status = executing; attach workflow contract summary and run_id.</td><td>Automatic for mana-backed or meaningful workflows.</td><td>Trivial chat may remain artifact-only.</td></tr>
docs/mana-next-runtime-event-mapping.md:28:<tr><td><code>contract.created</code></td><td>Attach workflow_contract_ref and summary fields.</td><td>Automatic for workflows with mana record.</td><td>Full contract lives in run artifacts.</td></tr>
docs/mana-next-runtime-event-mapping.md:29:<tr><td><code>tool.started</code></td><td>No mana write.</td><td>Never by default.</td><td>Trace only.</td></tr>
docs/mana-next-runtime-event-mapping.md:30:<tr><td><code>tool.completed</code></td><td>No mana write unless it produced an artifact/evidence ref.</td><td>Artifact summary only.</td><td>Example: diff.patch, evidence.md, verify.log.</td></tr>
docs/mana-next-runtime-event-mapping.md:37:<tr><td><code>mana.updated</code></td><td>No recursive write.</td><td>Never.</td><td>Trace only to avoid loops.</td></tr>
docs/mana-next-runtime-event-mapping.md:62:No mana writes:
docs/mana-next-runtime-event-mapping.md:88:mana update:
docs/mana-next-runtime-event-mapping.md:119:mana update:
docs/mana-next-runtime-event-mapping.md:141:mana update:
docs/mana-next-runtime-event-mapping.md:156:Runtime should batch/coalesce mana updates where possible:
docs/mana-next-runtime-event-mapping.md:162:This prevents `.mana` churn.
docs/mana-next-runtime-event-mapping.md:166:If mana update fails:
docs/mana-next-runtime-event-mapping.md:171:4. Keep run artifacts intact so a later repair can reconstruct mana refs.
docs/mana-next-runtime-event-mapping.md:178:- Should trivial TUI sessions ever create mana records automatically?
docs/workflow-first-ux.md:15:4. **Review and audit** — evidence packets, traces, mana ledger, runtime state.
docs/workflow-first-ux.md:16:5. **Use advanced workflows** — worktree-auto, TypeScript extensions, roles,
docs/workflow-first-ux.md:32:still work. Routine work does not require manually creating mana tasks, choosing
docs/workflow-first-ux.md:51:evidence packets, ReferenceMonitor policy records, verification gates, and mana
docs/workflow-first-ux.md:52:ledger notes. You do not need to manage those pieces for small tasks; they are
docs/workflow-first-ux.md:67:The goal is not to turn the TUI into a project-management dashboard. The chat is
docs/workflow-first-ux.md:125:Use mana when the work should survive beyond one chat turn. Ignore it for simple
docs/workflow-first-ux.md:174:## 9. TypeScript extensions
docs/workflow-first-ux.md:176:TypeScript extensions are manifest-declared tools. Rust still owns discovery,
docs/workflow-first-ux.md:208:should describe it as planned or advanced rather than implying every role flow is
docs/workflow-first-ux.md:226:### Do I need to learn mana?
docs/workflow-first-ux.md:228:No. Use mana when work is durable, multi-step, blocked, delegated, or needs
docs/workflow-first-ux.md:257:### Why was a TypeScript extension denied?
docs/mana-next-examples.md:1:# mana-next Workflow Ledger Examples
docs/mana-next-examples.md:4:Parent: mana `394.3` / child `394.3.7`
docs/mana-next-examples.md:6:These examples demonstrate the streamlined mana-next vocabulary while preserving compatibility with current mana concepts.
docs/mana-next-examples.md:146:## Current mana-compatible markdown sketch
docs/mana-next-examples.md:148:A current mana task can remain markdown/frontmatter and gain workflow-ledger refs later.
docs/mana-next-examples.md:184:~/.mana/ledger/
docs/mana-next-examples.md:191:The current mana markdown unit remains readable without these sidecars.
docs/release-promotions/commit-board.html:59:<script id="commit-data" type="application/json">[{&quot;sha&quot;: &quot;4e7f7464e1ef12b17dee43636fdfdebf8385ad59&quot;, &quot;short&quot;: &quot;4e7f746&quot;, &quot;subject&quot;: &quot;Reduce imp TUI startup latency&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-19T11:38:14-07:00&quot;, &quot;side&quot;: &quot;nightly-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-llm/src/auth.rs&quot;, &quot;crates/imp-llm/src/providers/openai.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 214, &quot;deletions&quot;: 43, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;moderate churn (257 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-llm/src/providers&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;d6521026f113f6fe80b5f55150cf66658190289f&quot;, &quot;short&quot;: &quot;d652102&quot;, &quot;subject&quot;: &quot;Prepare vanilla imp release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T14:02:13-07:00&quot;, &quot;side&quot;: &quot;nightly-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;.mana/.3-set-up-harbor-adapter-and-terminal-bench-20-runner.md&quot;, &quot;.mana/.5-add-safe-automatic-context-compaction-for-long-run.md&quot;, &quot;.mana/.5.1-add-disabled-by-default-auto-compaction-config-sca.md&quot;, &quot;.mana/.6-hardening-pass-reduce-bugs-and-contract-mismatches.md&quot;, &quot;.mana/.6.6-enforce-lua-extension-capability-boundaries.md&quot;, &quot;.mana/.6.7-propagate-cancellation-into-active-tool-execution.md&quot;, &quot;.mana/.6.8-align-diff-tool-registration-with-mode-contracts.md&quot;, &quot;.mana/.9-upgrade-imp-mana-authoring-prompt-contract-for-orc.md&quot;, &quot;.mana/.gitignore&quot;, &quot;.mana/21-imp-efficiency-smarter-tool-output-truncation.md&quot;, &quot;.mana/245.1-define-manaimp-contract-implications-of-file-nativ.md&quot;, &quot;.mana/245.1.1-define-vnext-manaimp-subagent-handoff-packet-for-o.md&quot;, &quot;.mana/248-comprehensive-imp-uiux-review-upgrade-and-polish-a.md&quot;, &quot;.mana/248.14-implement-restrained-ansi-emphasis-for-shell-typog.md&quot;, &quot;.mana/248.16.5-create-svg-wireframes-for-candidate-imp-tui-layout.md&quot;, &quot;.mana/248.16.7-revise-imp-tui-wireframes-and-core-memo-after-user.md&quot;, &quot;.mana/248.17-design-terminal-emulator-native-coding-agent-cockp.md&quot;, &quot;.mana/248.18-harden-and-humanize-imp-error-streaming-across-pro.md&quot;, &quot;.mana/248.18.1-extract-shared-imp-core-streamed-error-normalizati.md&quot;, &quot;.mana/248.18.2-harden-imp-core-partial-stream-and-silent-eof-diag.md&quot;, &quot;.mana/248.18.3-design-stable-machine-facing-streamed-error-envelo.md&quot;, &quot;.mana/248.7-plan-shared-uxruntime-seams-for-shell-tui-and-view.md&quot;, &quot;.mana/248.9-capture-and-sequence-real-user-feedback-on-the-new.md&quot;, &quot;.mana/249-reduce-duplicate-verbose-mana-change-narration-in.md&quot;, &quot;.mana/250-shape-getimpdev-landing-page-direction-and-impleme.md&quot;, &quot;.mana/254-fresh-smoke-trial-for-native-imp-run-on-an-isolate.md&quot;, &quot;.mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md&quot;, &quot;.mana/257-draft-imp-ontologymd-for-shared-featureruntime-lan.md&quot;, &quot;.mana/259-audit-panic-usage-and-detached-task-failure-policy.md&quot;, &quot;.mana/263-audit-and-isolate-library-level-stderr-writes-that.md&quot;, &quot;.mana/263.2-classify-mana-core-stderr-writes-for-embedded-risk.md&quot;, &quot;.mana/264-normalize-imp-storage-topology-for-sessions-config.md&quot;, &quot;.mana/264.1-audit-current-imp-durable-storage-surfaces-and-pat.md&quot;, &quot;.mana/264.2-specify-normalized-imp-storage-contract-and-migrat.md&quot;, &quot;.mana/264.3.1-add-shared-imp-core-storage-path-module-for-canoni.md&quot;, &quot;.mana/264.3.2-migrate-config-auth-session-and-session-search-cal.md&quot;, &quot;.mana/264.3.3-migrate-instruction-discovery-to-canonical-impagen.md&quot;, &quot;.mana/264.3.4-implement-non-destructive-migration-from-legacy-im.md&quot;, &quot;.mana/264.4-audit-and-fix-imp-session-index-lifecycle-wiring-f.md&quot;, &quot;.mana/264.6-decide-canonical-imp-filesystem-roots-for-global-a.md&quot;, &quot;.mana/264.7-specify-precedence-and-merge-rules-between-imp-and.md&quot;, &quot;.mana/264.8-specify-migration-from-xdgmacos-legacy-paths-into.md&quot;, &quot;.mana/266-cross-codebase-review-compare-imp-and-hermes-agent.md&quot;, &quot;.mana/266.1-design-adoption-path-provider-resilience-and-auth.md&quot;, &quot;.mana/266.2-design-adoption-path-session-recall-memory-and-con.md&quot;, &quot;.mana/266.3-design-adoption-path-extension-seams-and-product-s.md&quot;, &quot;.mana/266.4-imp-vnext-ranked-roadmap-and-phased-execution-plan.md&quot;, &quot;.mana/266.4.7-phase-5-epic-selective-later-product-surface-expan.md&quot;, &quot;.mana/267-adopt-highest-value-product-lessons-from-opencode.md&quot;, &quot;.mana/268.1-diagnose-native-imp-mana-tool-divergence-from-cli.md&quot;, &quot;.mana/27-improve-mana-pool-competitive-grade-dispatch-engin.md&quot;, &quot;.mana/27.14-define-attempt-scoped-autonomy-observation-record.md&quot;, &quot;.mana/27.2-imp-ui-compact-mana-statusprogress-surface.md&quot;, &quot;.mana/271-add-native-youtube-video-interpretation-support-to.md&quot;, &quot;.mana/271.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/271.2-harden-imp-spawn-and-mana-closetool-execution-agai.md&quot;, &quot;.mana/272-add-native-video-context-ingestion-architecture-fo.md&quot;, &quot;.mana/272.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/272.2-design-richer-video-interpretation-beyond-transcri.md&quot;, &quot;.mana/273-diagnose-and-harden-kimi-code-oauth-model-routing.md&quot;, &quot;.mana/273.5-sprint-import-and-execute-pi-typescript-extensions.md&quot;, &quot;.mana/273.5.10-prove-bun-ts-adapter-against-local-pi-color-palett.md&quot;, &quot;.mana/273.5.11-add-official-pi-dynamic-tools-compatibility-fixtur.md&quot;, &quot;.mana/273.5.12-define-sprint-1-typescriptpi-extension-support-bou.md&quot;, &quot;.mana/273.5.13-probe-dependency-bearing-pi-extension-compatibilit.md&quot;, &quot;.mana/273.5.4-normalize-typeboxjson-schemas-from-typescript-exte.md&quot;, &quot;.mana/275-assess-and-sequence-next-llm-oauth-provider-integr.md&quot;, &quot;.mana/275.10-inventory-pi-and-imp-provideroauth-surfaces.md&quot;, &quot;.mana/275.11-sequence-pi-provideroauth-parity-implementation.md&quot;, &quot;.mana/275.6-assess-pi-google-antigravity-provider-route-for-im.md&quot;, &quot;.mana/275.9-research-unofficial-cursor-provider-support-for-im.md&quot;, &quot;.mana/276-investigate-and-harden-tui-esc-cancellation-for-hu.md&quot;, &quot;.mana/277-fix-imp-tui-clean-ui-corruption-and-string-join-ov.md&quot;, &quot;.mana/278-fix-inspector-mode-interaction-model.md&quot;, &quot;.mana/28.1-make-imp-run-the-canonical-mana-worker-runtime-whi.md&quot;, &quot;.mana/28.5.1-patch-imp-system-prompt-with-mana-first-planning-d.md&quot;, &quot;.mana/28.5.6-implement-turn-scoped-mana-review-packet-aggregati.md&quot;, &quot;.mana/28.5.7-render-between-turn-mana-review-packets-across-imp.md&quot;, &quot;.mana/28.5.7.1-add-shared-imp-core-turnmanadelta-renderer-and-man.md&quot;, &quot;.mana/28.5.7.2-render-compact-between-turn-mana-block-and-textual.md&quot;, &quot;.mana/28.5.7.3-render-between-turn-mana-review-packets-in-imp-cli.md&quot;, &quot;.mana/28.5.7.4-add-shared-manareviewmode-config-and-presentation.md&quot;, &quot;.mana/28.5.7.5-wire-imp-tui-compact-widget-tray-block-and-sidebar.md&quot;, &quot;.mana/280-review-project-gaps-that-would-make-imp-stronger-t.md&quot;, &quot;.mana/280.1-run-dirac-evals-with-imp-using-gemini-secret.md&quot;, &quot;.mana/280.2-adopt-dirac-inspired-code-intelligence-and-precise.md&quot;, &quot;.mana/280.2.1.1-decide-migration-safe-naming-strategy-for-imp-scan.md&quot;, &quot;.mana/280.2.2-implement-read-oriented-symbol-extraction-and-skel.md&quot;, &quot;.mana/280.2.3-add-anchor-backed-read-and-stale-safe-edit-flow-to.md&quot;, &quot;.mana/280.2.4-implement-edit-transaction-batching-with-combined.md&quot;, &quot;.mana/282-design-native-scoped-secret-injection-for-command.md&quot;, &quot;.mana/285-verify-installed-imp-binary-includes-latest-secret.md&quot;, &quot;.mana/290-complete-imp-codebase-quality-audit.md&quot;, &quot;.mana/290.1-split-imp-tui-apprs-by-responsibility.md&quot;, &quot;.mana/290.2-split-imp-core-agentrs-into-focused-runtime-module.md&quot;, &quot;.mana/290.3-split-imp-cli-librs-into-command-modules.md&quot;, &quot;.mana/290.4-split-native-mana-tool-implementation-into-focused.md&quot;, &quot;.mana/291-rewrite-imp-readme-around-product-features-mana-an.md&quot;, &quot;.mana/31.2-add-guardrail-config-types-and-profile-selection-t.md&quot;, &quot;.mana/31.3-integrate-guardrails-into-the-imp-system-prompt-an.md&quot;, &quot;.mana/31.4-add-the-initial-zig-guardrail-profile-and-document.md&quot;, &quot;.mana/33-chat-view-replace-duplicated-animation-logic-with.md&quot;, &quot;.mana/34-sidebar-detail-header-use-spinnerframe-and-respect.md&quot;, &quot;.mana/35-editor-remove-dead-tick-and-animationlevel-params.md&quot;, &quot;.mana/36-animation-config-reconcile-minimal-namingdocs-afte.md&quot;, &quot;.mana/37-add-first-class-usage-accounting-and-reporting-to.md&quot;, &quot;.mana/37.5-test-and-document-imp-usage-accountingreporting.md&quot;, &quot;.mana/41-anthropic-api-parity-adopt-claude-code-patterns-fo.md&quot;, &quot;.mana/44-define-memory-and-code-intelligence-architecture-f.md&quot;, &quot;.mana/44.1-author-guest-runtime-extension-substrate-proposal.md&quot;, &quot;.mana/44.1.10-implement-documentworkspace-symbols-with-ast-first.md&quot;, &quot;.mana/44.1.11-implement-hover-and-signature-help-on-the-phase-1.md&quot;, &quot;.mana/44.1.12-unify-code-intelligence-diagnostic-summaries-with.md&quot;, &quot;.mana/44.1.14-evaluate-whether-repeated-evidence-promotion-flows.md&quot;, &quot;.mana/44.1.5-plan-guarded-write-oriented-semantic-actions-and-p.md&quot;, &quot;.mana/44.1.5.5-specify-semantic-write-execution-contract-for-prev.md&quot;, &quot;.mana/44.1.6-sequence-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.6.1-define-shared-normalization-envelopes-for-read-ori.md&quot;, &quot;.mana/44.1.6.2-plan-diagnostics-plus-ast-alignment-for-the-first.md&quot;, &quot;.mana/44.1.6.3-plan-document-symbols-and-go-to-definition-over-th.md&quot;, &quot;.mana/44.1.6.4-plan-references-and-workspace-symbol-browsing-for.md&quot;, &quot;.mana/44.1.6.5-plan-hover-and-signature-enrichment-after-core-rea.md&quot;, &quot;.mana/44.1.7-roll-out-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.8-normalize-read-oriented-code-intelligence-queryres.md&quot;, &quot;.mana/44.1.9-implement-phase-1-diagnostics-go-to-definition-and.md&quot;, &quot;.mana/44.3-translate-guest-runtime-design-into-phased-impleme.md&quot;, &quot;.mana/45-tower-rebuild-around-explicit-contracts-durable-le.md&quot;, &quot;.mana/45.10.5-update-docs-for-mana-platform-substrate-and-imp-pr.md&quot;, &quot;.mana/45.11-capture-near-term-imp-execution-lanes-under-the-im.md&quot;, &quot;.mana/45.11.1-resolve-consequential-defaults-for-near-term-imp-i.md&quot;, &quot;.mana/45.11.1.1-clarify-whether-native-rust-not-lua-applies-to-imp.md&quot;, &quot;.mana/45.11.1.2-sequence-near-term-imp-implementation-order-from-s.md&quot;, &quot;.mana/45.4-phase-3-introduce-runner-protocol-and-local-adapte.md&quot;, &quot;.mana/45.4.2-plan-the-first-imp-local-runner-adapter-that-consu.md&quot;, &quot;.mana/45.4.4-plan-the-cutover-from-current-imp-run-plus-mana-ru.md&quot;, &quot;.mana/45.5-phase-4-rebuild-imp-around-stable-workerruntime-se.md&quot;, &quot;.mana/45.5.1-map-imp-core-hotspots-into-target-runtime-context.md&quot;, &quot;.mana/45.5.3-write-a-compact-imp-decomposition-order-for-post-c.md&quot;, &quot;.mana/45.7-phase-6-harden-policy-isolation-and-migration-surf.md&quot;, &quot;.mana/45.7.4-evaluate-whether-imp-should-add-a-native-gitrepo-t.md&quot;, &quot;.mana/46-broaden-imp-attention-beyond-toolsprompting-under.md&quot;, &quot;.mana/46.1-reconcile-long-session-runtime-safety-gaps-and-tur.md&quot;, &quot;.mana/46.2-reconcile-user-visible-discoverability-and-ux-brea.md&quot;, &quot;.mana/46.2.1-surface-usage-reporting-in-the-tui-commandhelpstar.md&quot;, &quot;.mana/47-rebuild-imp-around-explicit-runtime-boundaries-pro.md&quot;, &quot;.mana/47.1-contracts-and-ownership-boundary-for-mana-imp-rebu.md&quot;, &quot;.mana/47.6-sequence-the-imp-rebuild-as-an-incremental-migrati.md&quot;, &quot;.mana/50-define-cli-first-operator-surface-for-imp-with-tui.md&quot;, &quot;.mana/50.10-implement-guided-cli-parity-flows-for-settings-per.md&quot;, &quot;.mana/50.10.1-implement-terminal-native-imp-settings-flow-for-cl.md&quot;, &quot;.mana/50.10.1.2-let-imp-chat-no-tools-reach-the-shell-without-prov.md&quot;, &quot;.mana/50.10.2-implement-terminal-native-imp-personality-flow-for.md&quot;, &quot;.mana/50.11-implement-first-shell-to-view-handoff-for-sessions.md&quot;, &quot;.mana/50.11.2-align-imp-chat-view-handoff-with-explicit-imp-view.md&quot;, &quot;.mana/50.12-flip-plain-imp-to-imp-chat-after-shell-readiness-g.md&quot;, &quot;.mana/50.13-plan-extraction-of-shared-fullscreen-consumed-runt.md&quot;, &quot;.mana/50.14-specify-the-shared-imp-ui-request-and-runtime-even.md&quot;, &quot;.mana/50.16-follow-on-cli-native-affordance-stack-after-505-se.md&quot;, &quot;.mana/50.16.1-define-stable-human-vs-machine-output-modes-across.md&quot;, &quot;.mana/50.16.2-plan-cli-first-checkpoint-productization-after-out.md&quot;, &quot;.mana/50.16.3-plan-visible-cli-first-planning-artifacts-and-exec.md&quot;, &quot;.mana/50.16.4-plan-first-class-approval-policy-layer-for-cli-fir.md&quot;, &quot;.mana/50.16.5-surface-session-browsing-and-session-search-as-fir.md&quot;, &quot;.mana/50.16.5.1-audit-and-reconcile-imp-session-storage-and-sessio.md&quot;, &quot;.mana/50.16.6-plan-detachedbackground-local-execution-after-cli.md&quot;, &quot;.mana/50.17-define-stable-human-vs-machine-output-contracts-fo.md&quot;, &quot;.mana/50.18-define-cli-first-session-browsing-and-sessionsearc.md&quot;, &quot;.mana/50.19-define-stable-imp-human-vs-machine-output-contract.md&quot;, &quot;.mana/50.20-plan-first-cli-first-checkpoint-productization-ove.md&quot;, &quot;.mana/50.21-specify-visible-planning-artifacts-and-checklist-b.md&quot;, &quot;.mana/50.22-specify-the-first-visible-planning-workflow-and-pl.md&quot;, &quot;.mana/50.23-specify-cli-first-approval-policy-and-blocked-stat.md&quot;, &quot;.mana/50.24-define-the-first-cli-first-approval-policy-surface.md&quot;, &quot;.mana/50.25-specify-detachedbackground-local-execution-contrac.md&quot;, &quot;.mana/50.26-define-the-first-local-detachedbackground-executio.md&quot;, &quot;.mana/50.6-design-the-cli-first-interactive-shell-path-for-im.md&quot;, &quot;.mana/50.9-implement-the-first-cli-first-proving-slice-with-e.md&quot;, &quot;.mana/51.6.1-audit-current-mana-core-embedding-surface-against.md&quot;, &quot;.mana/65-root-mana-currently-lists-child-513-but-direct-sho.md&quot;, &quot;.mana/69-imp-cli-no-longer-contains-duplicate-mana-unit-loa.md&quot;, &quot;.mana/73-code-intelligence-outputs-are-transient-by-default.md&quot;, &quot;.mana/81-design-imp-native-delegation-tool-around-imp-run-a.md&quot;, &quot;.mana/81.10-define-codemap-backed-context-seam-for-imp-run-and.md&quot;, &quot;.mana/82-assess-gpt-54-pro-support-through-openai-chatgpt-o.md&quot;, &quot;.mana/82.2-add-gpt-54-pro-to-imp-model-registry-only-after-oa.md&quot;, &quot;.mana/83-harden-imp-tui-text-box-cursor-and-bounds-handling.md&quot;, &quot;.mana/RULES.md&quot;, &quot;.mana/archive/2026/03/.2-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/16-imp-core-hardening-production-ready-agent-engine.md&quot;, &quot;.mana/archive/2026/03/16.1-wire-config-agent-agentbuilder-thresholds-hooks-re.md&quot;, &quot;.mana/archive/2026/03/16.2-tool-argument-validation-json-schema-before-execut.md&quot;, &quot;.mana/archive/2026/03/16.3-llm-retry-with-exponential-backoff-and-jitter.md&quot;, &quot;.mana/archive/2026/03/16.4-loop-detection-prevent-infinite-tool-call-retry-lo.md&quot;, &quot;.mana/archive/2026/03/16.5-file-not-found-suggestions-with-levenshtein-rankin.md&quot;, &quot;.mana/archive/2026/03/16.6-auto-resume-after-compaction-re-queue-original-pro.md&quot;, &quot;.mana/archive/2026/03/16.7-file-read-tracking-and-staleness-detection.md&quot;, &quot;.mana/archive/2026/03/16.8-file-version-history-pre-edit-snapshots-for-rollba.md&quot;, &quot;.mana/archive/2026/03/17-imp-efficiency-enable-prompt-caching.md&quot;, &quot;.mana/archive/2026/03/19-imp-efficiency-in-session-file-content-cache.md&quot;, &quot;.mana/archive/2026/03/20-imp-efficiency-parallelize-grep-block-search-with.md&quot;, &quot;.mana/archive/2026/03/229-imp-rust-coding-agent-engine.md&quot;, &quot;.mana/archive/2026/03/229.1-workspace-setup-imp-llm-types.md&quot;, &quot;.mana/archive/2026/03/229.10-imp-llm-anthropic-oauth.md&quot;, &quot;.mana/archive/2026/03/229.11-imp-core-hook-system.md&quot;, &quot;.mana/archive/2026/03/229.12-imp-core-tree-sitter-tools-probesearch-probeextrac.md&quot;, &quot;.mana/archive/2026/03/229.13-imp-core-config-resource-discovery.md&quot;, &quot;.mana/archive/2026/03/229.14-imp-core-system-prompt-assembly.md&quot;, &quot;.mana/archive/2026/03/229.15-imp-lua-lua-extension-runtime.md&quot;, &quot;.mana/archive/2026/03/229.16-imp-core-shell-tool-loader.md&quot;, &quot;.mana/archive/2026/03/229.17-imp-tui-ratatui-interactive-mode.md&quot;, &quot;.mana/archive/2026/03/229.18-imp-cli-binary-entry-point.md&quot;, &quot;.mana/archive/2026/03/229.2-imp-llm-anthropic-provider.md&quot;, &quot;.mana/archive/2026/03/229.3-imp-core-tool-trait-file-tools-read-write-edit-mul.md&quot;, &quot;.mana/archive/2026/03/229.4-imp-core-bash-grep-find-tools.md&quot;, &quot;.mana/archive/2026/03/229.5-imp-core-ask-diff-tools.md&quot;, &quot;.mana/archive/2026/03/229.6-imp-core-agent-loop.md&quot;, &quot;.mana/archive/2026/03/229.7-imp-core-session-manager.md&quot;, &quot;.mana/archive/2026/03/229.8-imp-core-context-management-observation-masking-co.md&quot;, &quot;.mana/archive/2026/03/229.9-imp-llm-openai-google-providers.md&quot;, &quot;.mana/archive/2026/03/23-learning-loop-agent-curated-memory-skill-managemen.md&quot;, &quot;.mana/archive/2026/03/23.1-system-prompt-layer-6-wire-memory-into-prompt-asse.md&quot;, &quot;.mana/archive/2026/03/23.2-memory-store-and-memory-tool.md&quot;, &quot;.mana/archive/2026/03/23.3-skill-manage-tool-agent-creates-patches-and-delete.md&quot;, &quot;.mana/archive/2026/03/23.4-learning-nudges-system-prompt-text-and-onagentend.md&quot;, &quot;.mana/archive/2026/03/23.5-session-index-with-fts5-full-text-search.md&quot;, &quot;.mana/archive/2026/03/23.6-session-search-tool.md&quot;, &quot;.mana/archive/2026/03/24-tui-ux-overhaul-information-density-summaries-inte.md&quot;, &quot;.mana/archive/2026/03/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/03/24.2-progress-indicator-in-status-bar-during-streaming.md&quot;, &quot;.mana/archive/2026/03/24.3-per-tool-call-expandcollapse-and-auto-expand-error.md&quot;, &quot;.mana/archive/2026/03/24.4-turn-end-summary-with-file-change-tracking.md&quot;, &quot;.mana/archive/2026/03/24.5-visual-separation-of-tool-activity-from-assistant.md&quot;, &quot;.mana/archive/2026/03/24.6-editor-polish-placeholder-model-indicator-keybindi.md&quot;, &quot;.mana/archive/2026/03/24.7-fix-context-window-tracking-use-actual-conversatio.md&quot;, &quot;.mana/archive/2026/03/24.8-approval-flow-wire-userinterface-for-tool-confirma.md&quot;, &quot;.mana/archive/2026/03/25-multi-provider-llm-support-with-data-driven-welcom.md&quot;, &quot;.mana/archive/2026/03/25.1-provider-metadata-registry-auth-generalization.md&quot;, &quot;.mana/archive/2026/03/25.2-openai-compatible-chat-completions-provider.md&quot;, &quot;.mana/archive/2026/03/25.3-add-builtin-models-for-new-providers.md&quot;, &quot;.mana/archive/2026/03/25.4-data-driven-welcome-flow.md&quot;, &quot;.mana/archive/2026/03/25.5-generalize-cli-login-for-all-providers.md&quot;, &quot;.mana/archive/2026/03/26-fix-imp-tui-compile-errors-around-toolcallorder-re.md&quot;, &quot;.mana/archive/2026/03/27.1-imp-core-mana-tool-add-native-orchestration-action.md&quot;, &quot;.mana/archive/2026/03/31-add-configurable-engineering-guardrails-to-imp.md&quot;, &quot;.mana/archive/2026/03/37.1-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/37.2-persist-canonical-usage-entries-in-imp-core-sessio.md&quot;, &quot;.mana/archive/2026/03/37.3-unify-usage-persistence-across-imp-execution-paths.md&quot;, &quot;.mana/archive/2026/03/37.4-add-imp-usage-reporting-commands-and-export.md&quot;, &quot;.mana/archive/2026/04/.10-define-clean-mana-vs-imp-boundary-and-memory-conso.md&quot;, &quot;.mana/archive/2026/04/.10.1-define-imp-memory-layer-architecture-and-mana-ownership-boundaries.md&quot;, &quot;.mana/archive/2026/04/.10.2-design-a-mana-wiki-schema-and-knowledge-maintenance-workflow.md&quot;, &quot;.mana/archive/2026/04/.10.3-strengthen-mana-first-prompt-doctrine-for-durable-planning.md&quot;, &quot;.mana/archive/2026/04/.10.4-design-mana-aware-runtime-context-read-path-for-prompt-assembly.md&quot;, &quot;.mana/archive/2026/04/.10.5-design-inline-mana-state-and-knowledge-surfaces-for-imp-runtime.md&quot;, &quot;.mana/archive/2026/04/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/04/266.4.3.4-fix-stale-secret-metadata-and-missing-keychain-dia.md&quot;, &quot;.mana/archive/2026/04/27.4-imp-promptingtool-guidance-prefer-native-mana-tool.md&quot;, &quot;.mana/archive/2026/04/272-add-kimi-model-compatibility-and-fix-ctrll-model-p.md&quot;, &quot;.mana/archive/2026/04/274-audit-and-simplify-imp-core-config-module.md&quot;, &quot;.mana/archive/2026/04/28-surface-built-in-features-already-implemented-in-i.md&quot;, &quot;.mana/archive/2026/04/28.1.1-specify-the-strengthened-imp-run-worker-contract-a.md&quot;, &quot;.mana/archive/2026/04/28.1.2-implement-reusable-imp-side-mana-unit-worker-runti.md&quot;, &quot;.mana/archive/2026/04/28.1.3-integrate-mana-run-with-the-strengthened-imp-run-w.md&quot;, &quot;.mana/archive/2026/04/28.1.5-fix-native-imp-delegate-worker-defaults-for-openai.md&quot;, &quot;.mana/archive/2026/04/28.1.5-make-imps-native-mana-tool-the-clear-first-class-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.2-fix-direct-imp-run-codexopenai-worker-request-defa.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.2-extract-shared-model-first-runtime-connection-reso.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.3-refactor-headless-worker-auth-to-normalize-empty-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.4-clarify-imp-to-imp-tool-vocabulary-and-align-docs.md&quot;, &quot;.mana/archive/2026/04/29.3-add-recent-session-previews-to-the-imp-startup-pan.md&quot;, &quot;.mana/archive/2026/04/29.4-add-context-aware-quickstart-guidance-and-health-s.md&quot;, &quot;.mana/archive/2026/04/29.6.1-implement-native-mana-scope-targeting-in-imp-tool.md&quot;, &quot;.mana/archive/2026/04/29.6.2-implement-safe-partial-mana-update-semantics-in-im.md&quot;, &quot;.mana/archive/2026/04/29.6.3-implement-append-style-mana-actions-for-conversati.md&quot;, &quot;.mana/archive/2026/04/30-render-compact-widgetstatus-surfaces-already-suppo.md&quot;, &quot;.mana/archive/2026/04/31.1-write-the-engineering-guardrails-design-note-for-i.md&quot;, &quot;.mana/archive/2026/04/32-productize-checkpoints-from-imps-existing-file-sna.md&quot;, &quot;.mana/archive/2026/04/32.1-checkpoint-foundation-shared-filehistory-wiring-an.md&quot;, &quot;.mana/archive/2026/04/32.2-checkpoint-persistence-session-custom-records-plus.md&quot;, &quot;.mana/archive/2026/04/32.3-checkpoint-ux-minimal-slash-command-list-and-resto.md&quot;, &quot;.mana/archive/2026/04/42-per-agent-cached-context-assembly-for-mana-dispatc.md&quot;, &quot;.mana/archive/2026/04/47.1.4-implement-the-first-shared-verifier-and-evidence-r.md&quot;, &quot;.mana/index.yaml.old&quot;, &quot;.mana/migration-conflicts/.3-add-secure-generic-credential-storage-and-lua-secr.md.txt&quot;, &quot;.mana/migration-conflicts/267-fix-native-imp-worker-openai-route-failure-when-sp.md.txt&quot;, &quot;.mana/migration-conflicts/27-native-mana-tool-overhaul-background-runs-lightwei.md.txt&quot;, &quot;.mana/migration-conflicts/270-make-uu-install-support-active-shell-binary-repair.md.txt&quot;, &quot;.mana/migration-conflicts/270.1-make-uu-install-complete-the-active-shell-imp-upgr.md.txt&quot;, &quot;.mana/migration-conflicts/271-harden-spawn-and-mana-tool-termination-so-closespa.md.txt&quot;, &quot;.mana/migration-conflicts/271.1-diagnose-hang-paths-in-imp-spawn-and-mana-closetoo.md.txt&quot;, &quot;.mana/migration-conflicts/273-make-pi-typescript-extensions-importable-into-imp.md.txt&quot;, &quot;.mana/migration-conflicts/275-rethink-imp-tui-tool-call-presentation-and-sidebar.md.txt&quot;, &quot;.mana/migration-conflicts/44-rethink-imp-extensions-as-guest-runtimes-with-opti.md.txt&quot;, &quot;.mana/migration-conflicts/44.1-plan-phased-implementation-of-imp-native-code-inte.md.txt&quot;, &quot;.mana/migration-conflicts/45-explore-ast-backed-symbolic-plan-layer-for-imp.md.txt&quot;, &quot;.mana/migration-conflicts/51-easy-fix-impmana-gaps-triaged-from-repo-scan.md.txt&quot;, &quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;README.md&quot;, &quot;crates/imp-cli/auth.json&quot;, &quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/Cargo.toml&quot;, &quot;crates/imp-core/skills/lua-tools/SKILL.md&quot;, &quot;crates/imp-core/skills/writing-skills/REFERENCE.md&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/import.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/sdk.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/bun_runner.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/discovery.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/pi_compat.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/schema.rs&quot;], &quot;insertions&quot;: 21, &quot;deletions&quot;: 29398, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;mostly README&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;, &quot;very high churn (29419 lines)&quot;]}, {&quot;sha&quot;: &quot;34f8be6671f5091d82792eff6ab9bba4ee34f6df&quot;, &quot;short&quot;: &quot;34f8be6&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T12:27:45-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.gitleaks.toml&quot;, &quot;Cargo.toml&quot;, &quot;crates/imp-cli/.gitignore&quot;, &quot;crates/imp-cli/Cargo.toml&quot;], &quot;insertions&quot;: 15, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 10, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;2c50e9633a829dec714836848a9faa3da14c7014&quot;, &quot;short&quot;: &quot;2c50e96&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T11:55:43-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.github/workflows/edge.yml&quot;, &quot;.github/workflows/release.yml&quot;], &quot;insertions&quot;: 2, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 13, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;risky subject keyword&quot;, &quot;touches .github/workflows&quot;]}, {&quot;sha&quot;: &quot;d36a3c1142af4797684158f90dc65d1a44357655&quot;, &quot;short&quot;: &quot;d36a3c1&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T10:11:53-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-cli/src/usage_report.rs&quot;, &quot;crates/imp-core/examples/sdk_session.rs&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/error_display.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/personality.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/session.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/web/read.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-lua/src/lib.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/terminal.rs&quot;, &quot;crates/imp-tui/src/views/ask_bar.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/sidebar.rs&quot;, &quot;crates/imp-tui/src/views/startup.rs&quot;, &quot;crates/imp-tui/src/views/tool_output.rs&quot;], &quot;insertions&quot;: 209, &quot;deletions&quot;: 211, &quot;risk_score&quot;: 44, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;moderate churn (420 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;371150fdaca0c02e3140222f84c03c6135153840&quot;, &quot;short&quot;: &quot;371150f&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T09:19:21-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;README.md&quot;], &quot;insertions&quot;: 124, &quot;deletions&quot;: 311, &quot;risk_score&quot;: 7, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;moderate churn (435 lines)&quot;, &quot;mostly README&quot;, &quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;b472eadd5b6afbe7a4a06aa7ec603043031f578b&quot;, &quot;short&quot;: &quot;b472ead&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-18T07:52:46-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;README.md&quot;], &quot;insertions&quot;: 21, &quot;deletions&quot;: 21, &quot;risk_score&quot;: 12, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;mostly README&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;42634dbe7b8171671fcef2063b765fe8284f93c0&quot;, &quot;short&quot;: &quot;42634db&quot;, &quot;subject&quot;: &quot;Merge branch &#x27;nightly&#x27; into release&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-17T18:30:33-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 2, &quot;files&quot;: [&quot;.gitignore&quot;, &quot;AGENTS.md&quot;, &quot;CHANGELOG.md&quot;, &quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;Cargo.workspace.toml&quot;, &quot;LICENSE&quot;, &quot;README.md&quot;, &quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/Cargo.toml&quot;, &quot;crates/imp-core/examples/tool_surface_live.rs&quot;, &quot;crates/imp-core/skills/writing-skills/REFERENCE.md&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/loop_policy.rs&quot;, &quot;crates/imp-core/src/agent/loop_state.rs&quot;, &quot;crates/imp-core/src/agent/mana_loop.rs&quot;, &quot;crates/imp-core/src/{agent.rs =&gt; agent/mod.rs}&quot;, &quot;crates/imp-core/src/agent/recovery.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/agent/turn_assessment.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/config.rs&quot;, &quot;crates/imp-core/src/context_prefill.rs&quot;, &quot;crates/imp-core/src/contracts.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/guardrails.rs&quot;, &quot;crates/imp-core/src/imp_session.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_next/ledger.rs&quot;, &quot;crates/imp-core/src/mana_next/mod.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/mana_run_state.rs&quot;, &quot;crates/imp-core/src/mana_worker.rs&quot;, &quot;crates/imp-core/src/policy.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/resources.rs&quot;, &quot;crates/imp-core/src/retry.rs&quot;, &quot;crates/imp-core/src/roles.rs&quot;, &quot;crates/imp-core/src/run_evidence.rs&quot;, &quot;crates/imp-core/src/session.rs&quot;, &quot;crates/imp-core/src/storage.rs&quot;, &quot;crates/imp-core/src/system_prompt.rs&quot;, &quot;crates/imp-core/src/tools/ask.rs&quot;, &quot;crates/imp-core/src/tools/bash.rs&quot;, &quot;crates/imp-core/src/tools/edit.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/git.rs&quot;, &quot;crates/imp-core/src/tools/imp.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/memory.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/tools/multi_edit.rs&quot;, &quot;crates/imp-core/src/tools/read.rs&quot;, &quot;crates/imp-core/src/tools/scan/kotlin.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/session_search.rs&quot;, &quot;crates/imp-core/src/tools/shell.rs&quot;, &quot;crates/imp-core/src/tools/web/github.rs&quot;, &quot;crates/imp-core/src/tools/web/mod.rs&quot;, &quot;crates/imp-core/src/tools/web/read.rs&quot;, &quot;crates/imp-core/src/tools/web/search.rs&quot;, &quot;crates/imp-core/src/tools/web/types.rs&quot;, &quot;crates/imp-core/src/tools/web/youtube.rs&quot;, &quot;crates/imp-core/src/tools/worktree.rs&quot;, &quot;crates/imp-core/src/tools/write.rs&quot;, &quot;crates/imp-core/src/trace.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/ui.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-gui/Cargo.toml&quot;, &quot;crates/imp-gui/README.md&quot;, &quot;crates/imp-gui/src/lib.rs&quot;, &quot;crates/imp-gui/src/main.rs&quot;, &quot;crates/imp-llm/Cargo.toml&quot;, &quot;crates/imp-llm/src/lib.rs&quot;, &quot;crates/imp-llm/src/provider.rs&quot;, &quot;crates/imp-llm/src/providers/anthropic.rs&quot;, &quot;crates/imp-llm/src/providers/openai.rs&quot;, &quot;crates/imp-lua/src/bridge.rs&quot;, &quot;crates/imp-lua/src/lib.rs&quot;, &quot;crates/imp-lua/src/loader.rs&quot;, &quot;crates/imp-lua/src/sandbox.rs&quot;, &quot;crates/imp-tui/Cargo.toml&quot;, &quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/event_source.rs&quot;, &quot;crates/imp-tui/src/keybindings.rs&quot;, &quot;crates/imp-tui/src/lib.rs&quot;, &quot;crates/imp-tui/src/terminal.rs&quot;, &quot;crates/imp-tui/src/tui_interface.rs&quot;, &quot;crates/imp-tui/src/turn_tracker.rs&quot;, &quot;crates/imp-tui/src/views/ask_bar.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;, &quot;crates/imp-tui/src/views/command_palette.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/file_finder.rs&quot;, &quot;crates/imp-tui/src/views/mana_navigator.rs&quot;, &quot;crates/imp-tui/src/views/mod.rs&quot;, &quot;crates/imp-tui/src/views/session_picker.rs&quot;, &quot;crates/imp-tui/src/views/settings.rs&quot;, &quot;crates/imp-tui/src/views/sidebar.rs&quot;, &quot;crates/imp-tui/src/views/startup.rs&quot;, &quot;crates/imp-tui/src/views/tool_output.rs&quot;, &quot;crates/imp-tui/src/views/tools.rs&quot;, &quot;docs/autonomy-modes.md&quot;, &quot;docs/design/lua-programmatic-interactions.md&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;, &quot;docs/mana-next-compatibility-adapter.md&quot;, &quot;docs/mana-next-examples.md&quot;, &quot;docs/mana-next-migration-test-plan.md&quot;, &quot;docs/mana-next-runtime-event-mapping.md&quot;, &quot;docs/mana-next-storage-strategy.md&quot;, &quot;docs/mana-next-ux.md&quot;, &quot;docs/mana-next-workflow-ledger.md&quot;, &quot;docs/reference-monitor-policy.md&quot;, &quot;docs/run-evidence.md&quot;, &quot;docs/trace-and-evidence-format.md&quot;, &quot;docs/trust-labels-and-provenance.md&quot;, &quot;docs/tui-workflow-wireframes.md&quot;, &quot;docs/verification-gates.md&quot;, &quot;docs/worktree-auto.md&quot;, &quot;imp-gui-wireframe.html&quot;], &quot;insertions&quot;: 39025, &quot;deletions&quot;: 5869, &quot;risk_score&quot;: 56, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;merge commit&quot;, &quot;mostly CHANGELOG&quot;, &quot;mostly README&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/mana_worker&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-llm/src/providers&quot;, &quot;touches crates/imp-tui/src/app&quot;, &quot;touches crates/imp-tui/src/event_source&quot;, &quot;very high churn (44894 lines)&quot;]}, {&quot;sha&quot;: &quot;eb3f46fb52a4b11228cf0df7d889a2d40e845980&quot;, &quot;short&quot;: &quot;eb3f46f&quot;, &quot;subject&quot;: &quot;Use published mana crates for release build&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-01T15:40:38-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;Cargo.lock&quot;, &quot;Cargo.toml&quot;, &quot;crates/imp-core/Cargo.toml&quot;], &quot;insertions&quot;: 8, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;touches Cargo.lock&quot;, &quot;touches Cargo.toml&quot;]}, {&quot;sha&quot;: &quot;9e6cd9c85b0da3cc2b93bed18a476e265ad719bb&quot;, &quot;short&quot;: &quot;9e6cd9c&quot;, &quot;subject&quot;: &quot;Clean release branch artifacts&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-01T14:12:24-07:00&quot;, &quot;side&quot;: &quot;release-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;.gitignore&quot;, &quot;.mana/.3-set-up-harbor-adapter-and-terminal-bench-20-runner.md&quot;, &quot;.mana/.5-add-safe-automatic-context-compaction-for-long-run.md&quot;, &quot;.mana/.5.1-add-disabled-by-default-auto-compaction-config-sca.md&quot;, &quot;.mana/.6-hardening-pass-reduce-bugs-and-contract-mismatches.md&quot;, &quot;.mana/.6.6-enforce-lua-extension-capability-boundaries.md&quot;, &quot;.mana/.6.7-propagate-cancellation-into-active-tool-execution.md&quot;, &quot;.mana/.6.8-align-diff-tool-registration-with-mode-contracts.md&quot;, &quot;.mana/.9-upgrade-imp-mana-authoring-prompt-contract-for-orc.md&quot;, &quot;.mana/.gitignore&quot;, &quot;.mana/21-imp-efficiency-smarter-tool-output-truncation.md&quot;, &quot;.mana/245.1-define-manaimp-contract-implications-of-file-nativ.md&quot;, &quot;.mana/245.1.1-define-vnext-manaimp-subagent-handoff-packet-for-o.md&quot;, &quot;.mana/248-comprehensive-imp-uiux-review-upgrade-and-polish-a.md&quot;, &quot;.mana/248.14-implement-restrained-ansi-emphasis-for-shell-typog.md&quot;, &quot;.mana/248.16.5-create-svg-wireframes-for-candidate-imp-tui-layout.md&quot;, &quot;.mana/248.16.7-revise-imp-tui-wireframes-and-core-memo-after-user.md&quot;, &quot;.mana/248.17-design-terminal-emulator-native-coding-agent-cockp.md&quot;, &quot;.mana/248.18-harden-and-humanize-imp-error-streaming-across-pro.md&quot;, &quot;.mana/248.18.1-extract-shared-imp-core-streamed-error-normalizati.md&quot;, &quot;.mana/248.18.2-harden-imp-core-partial-stream-and-silent-eof-diag.md&quot;, &quot;.mana/248.18.3-design-stable-machine-facing-streamed-error-envelo.md&quot;, &quot;.mana/248.7-plan-shared-uxruntime-seams-for-shell-tui-and-view.md&quot;, &quot;.mana/248.9-capture-and-sequence-real-user-feedback-on-the-new.md&quot;, &quot;.mana/249-reduce-duplicate-verbose-mana-change-narration-in.md&quot;, &quot;.mana/250-shape-getimpdev-landing-page-direction-and-impleme.md&quot;, &quot;.mana/254-fresh-smoke-trial-for-native-imp-run-on-an-isolate.md&quot;, &quot;.mana/256-run-one-shot-native-imp-print-smoke-before-imp-run.md&quot;, &quot;.mana/257-draft-imp-ontologymd-for-shared-featureruntime-lan.md&quot;, &quot;.mana/259-audit-panic-usage-and-detached-task-failure-policy.md&quot;, &quot;.mana/263-audit-and-isolate-library-level-stderr-writes-that.md&quot;, &quot;.mana/263.2-classify-mana-core-stderr-writes-for-embedded-risk.md&quot;, &quot;.mana/264-normalize-imp-storage-topology-for-sessions-config.md&quot;, &quot;.mana/264.1-audit-current-imp-durable-storage-surfaces-and-pat.md&quot;, &quot;.mana/264.2-specify-normalized-imp-storage-contract-and-migrat.md&quot;, &quot;.mana/264.3.1-add-shared-imp-core-storage-path-module-for-canoni.md&quot;, &quot;.mana/264.3.2-migrate-config-auth-session-and-session-search-cal.md&quot;, &quot;.mana/264.3.3-migrate-instruction-discovery-to-canonical-impagen.md&quot;, &quot;.mana/264.3.4-implement-non-destructive-migration-from-legacy-im.md&quot;, &quot;.mana/264.4-audit-and-fix-imp-session-index-lifecycle-wiring-f.md&quot;, &quot;.mana/264.6-decide-canonical-imp-filesystem-roots-for-global-a.md&quot;, &quot;.mana/264.7-specify-precedence-and-merge-rules-between-imp-and.md&quot;, &quot;.mana/264.8-specify-migration-from-xdgmacos-legacy-paths-into.md&quot;, &quot;.mana/266-cross-codebase-review-compare-imp-and-hermes-agent.md&quot;, &quot;.mana/266.1-design-adoption-path-provider-resilience-and-auth.md&quot;, &quot;.mana/266.2-design-adoption-path-session-recall-memory-and-con.md&quot;, &quot;.mana/266.3-design-adoption-path-extension-seams-and-product-s.md&quot;, &quot;.mana/266.4-imp-vnext-ranked-roadmap-and-phased-execution-plan.md&quot;, &quot;.mana/266.4.7-phase-5-epic-selective-later-product-surface-expan.md&quot;, &quot;.mana/267-adopt-highest-value-product-lessons-from-opencode.md&quot;, &quot;.mana/268.1-diagnose-native-imp-mana-tool-divergence-from-cli.md&quot;, &quot;.mana/27-improve-mana-pool-competitive-grade-dispatch-engin.md&quot;, &quot;.mana/27.14-define-attempt-scoped-autonomy-observation-record.md&quot;, &quot;.mana/27.2-imp-ui-compact-mana-statusprogress-surface.md&quot;, &quot;.mana/271-add-native-youtube-video-interpretation-support-to.md&quot;, &quot;.mana/271.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/271.2-harden-imp-spawn-and-mana-closetool-execution-agai.md&quot;, &quot;.mana/272-add-native-video-context-ingestion-architecture-fo.md&quot;, &quot;.mana/272.1-implement-pure-http-youtube-transcript-extraction.md&quot;, &quot;.mana/272.2-design-richer-video-interpretation-beyond-transcri.md&quot;, &quot;.mana/273-diagnose-and-harden-kimi-code-oauth-model-routing.md&quot;, &quot;.mana/273.5-sprint-import-and-execute-pi-typescript-extensions.md&quot;, &quot;.mana/273.5.10-prove-bun-ts-adapter-against-local-pi-color-palett.md&quot;, &quot;.mana/273.5.11-add-official-pi-dynamic-tools-compatibility-fixtur.md&quot;, &quot;.mana/273.5.12-define-sprint-1-typescriptpi-extension-support-bou.md&quot;, &quot;.mana/273.5.13-probe-dependency-bearing-pi-extension-compatibilit.md&quot;, &quot;.mana/273.5.4-normalize-typeboxjson-schemas-from-typescript-exte.md&quot;, &quot;.mana/275-assess-and-sequence-next-llm-oauth-provider-integr.md&quot;, &quot;.mana/275.10-inventory-pi-and-imp-provideroauth-surfaces.md&quot;, &quot;.mana/275.11-sequence-pi-provideroauth-parity-implementation.md&quot;, &quot;.mana/275.6-assess-pi-google-antigravity-provider-route-for-im.md&quot;, &quot;.mana/275.9-research-unofficial-cursor-provider-support-for-im.md&quot;, &quot;.mana/276-investigate-and-harden-tui-esc-cancellation-for-hu.md&quot;, &quot;.mana/277-fix-imp-tui-clean-ui-corruption-and-string-join-ov.md&quot;, &quot;.mana/278-fix-inspector-mode-interaction-model.md&quot;, &quot;.mana/28.1-make-imp-run-the-canonical-mana-worker-runtime-whi.md&quot;, &quot;.mana/28.5.1-patch-imp-system-prompt-with-mana-first-planning-d.md&quot;, &quot;.mana/28.5.6-implement-turn-scoped-mana-review-packet-aggregati.md&quot;, &quot;.mana/28.5.7-render-between-turn-mana-review-packets-across-imp.md&quot;, &quot;.mana/28.5.7.1-add-shared-imp-core-turnmanadelta-renderer-and-man.md&quot;, &quot;.mana/28.5.7.2-render-compact-between-turn-mana-block-and-textual.md&quot;, &quot;.mana/28.5.7.3-render-between-turn-mana-review-packets-in-imp-cli.md&quot;, &quot;.mana/28.5.7.4-add-shared-manareviewmode-config-and-presentation.md&quot;, &quot;.mana/28.5.7.5-wire-imp-tui-compact-widget-tray-block-and-sidebar.md&quot;, &quot;.mana/280-review-project-gaps-that-would-make-imp-stronger-t.md&quot;, &quot;.mana/280.1-run-dirac-evals-with-imp-using-gemini-secret.md&quot;, &quot;.mana/280.2-adopt-dirac-inspired-code-intelligence-and-precise.md&quot;, &quot;.mana/280.2.1.1-decide-migration-safe-naming-strategy-for-imp-scan.md&quot;, &quot;.mana/280.2.2-implement-read-oriented-symbol-extraction-and-skel.md&quot;, &quot;.mana/280.2.3-add-anchor-backed-read-and-stale-safe-edit-flow-to.md&quot;, &quot;.mana/280.2.4-implement-edit-transaction-batching-with-combined.md&quot;, &quot;.mana/282-design-native-scoped-secret-injection-for-command.md&quot;, &quot;.mana/285-verify-installed-imp-binary-includes-latest-secret.md&quot;, &quot;.mana/290-complete-imp-codebase-quality-audit.md&quot;, &quot;.mana/290.1-split-imp-tui-apprs-by-responsibility.md&quot;, &quot;.mana/290.2-split-imp-core-agentrs-into-focused-runtime-module.md&quot;, &quot;.mana/290.3-split-imp-cli-librs-into-command-modules.md&quot;, &quot;.mana/290.4-split-native-mana-tool-implementation-into-focused.md&quot;, &quot;.mana/291-rewrite-imp-readme-around-product-features-mana-an.md&quot;, &quot;.mana/31.2-add-guardrail-config-types-and-profile-selection-t.md&quot;, &quot;.mana/31.3-integrate-guardrails-into-the-imp-system-prompt-an.md&quot;, &quot;.mana/31.4-add-the-initial-zig-guardrail-profile-and-document.md&quot;, &quot;.mana/33-chat-view-replace-duplicated-animation-logic-with.md&quot;, &quot;.mana/34-sidebar-detail-header-use-spinnerframe-and-respect.md&quot;, &quot;.mana/35-editor-remove-dead-tick-and-animationlevel-params.md&quot;, &quot;.mana/36-animation-config-reconcile-minimal-namingdocs-afte.md&quot;, &quot;.mana/37-add-first-class-usage-accounting-and-reporting-to.md&quot;, &quot;.mana/37.5-test-and-document-imp-usage-accountingreporting.md&quot;, &quot;.mana/41-anthropic-api-parity-adopt-claude-code-patterns-fo.md&quot;, &quot;.mana/44-define-memory-and-code-intelligence-architecture-f.md&quot;, &quot;.mana/44.1-author-guest-runtime-extension-substrate-proposal.md&quot;, &quot;.mana/44.1.10-implement-documentworkspace-symbols-with-ast-first.md&quot;, &quot;.mana/44.1.11-implement-hover-and-signature-help-on-the-phase-1.md&quot;, &quot;.mana/44.1.12-unify-code-intelligence-diagnostic-summaries-with.md&quot;, &quot;.mana/44.1.14-evaluate-whether-repeated-evidence-promotion-flows.md&quot;, &quot;.mana/44.1.5-plan-guarded-write-oriented-semantic-actions-and-p.md&quot;, &quot;.mana/44.1.5.5-specify-semantic-write-execution-contract-for-prev.md&quot;, &quot;.mana/44.1.6-sequence-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.6.1-define-shared-normalization-envelopes-for-read-ori.md&quot;, &quot;.mana/44.1.6.2-plan-diagnostics-plus-ast-alignment-for-the-first.md&quot;, &quot;.mana/44.1.6.3-plan-document-symbols-and-go-to-definition-over-th.md&quot;, &quot;.mana/44.1.6.4-plan-references-and-workspace-symbol-browsing-for.md&quot;, &quot;.mana/44.1.6.5-plan-hover-and-signature-enrichment-after-core-rea.md&quot;, &quot;.mana/44.1.7-roll-out-phase-1-read-oriented-imp-code-intelligen.md&quot;, &quot;.mana/44.1.8-normalize-read-oriented-code-intelligence-queryres.md&quot;, &quot;.mana/44.1.9-implement-phase-1-diagnostics-go-to-definition-and.md&quot;, &quot;.mana/44.3-translate-guest-runtime-design-into-phased-impleme.md&quot;, &quot;.mana/45-tower-rebuild-around-explicit-contracts-durable-le.md&quot;, &quot;.mana/45.10.5-update-docs-for-mana-platform-substrate-and-imp-pr.md&quot;, &quot;.mana/45.11-capture-near-term-imp-execution-lanes-under-the-im.md&quot;, &quot;.mana/45.11.1-resolve-consequential-defaults-for-near-term-imp-i.md&quot;, &quot;.mana/45.11.1.1-clarify-whether-native-rust-not-lua-applies-to-imp.md&quot;, &quot;.mana/45.11.1.2-sequence-near-term-imp-implementation-order-from-s.md&quot;, &quot;.mana/45.4-phase-3-introduce-runner-protocol-and-local-adapte.md&quot;, &quot;.mana/45.4.2-plan-the-first-imp-local-runner-adapter-that-consu.md&quot;, &quot;.mana/45.4.4-plan-the-cutover-from-current-imp-run-plus-mana-ru.md&quot;, &quot;.mana/45.5-phase-4-rebuild-imp-around-stable-workerruntime-se.md&quot;, &quot;.mana/45.5.1-map-imp-core-hotspots-into-target-runtime-context.md&quot;, &quot;.mana/45.5.3-write-a-compact-imp-decomposition-order-for-post-c.md&quot;, &quot;.mana/45.7-phase-6-harden-policy-isolation-and-migration-surf.md&quot;, &quot;.mana/45.7.4-evaluate-whether-imp-should-add-a-native-gitrepo-t.md&quot;, &quot;.mana/46-broaden-imp-attention-beyond-toolsprompting-under.md&quot;, &quot;.mana/46.1-reconcile-long-session-runtime-safety-gaps-and-tur.md&quot;, &quot;.mana/46.2-reconcile-user-visible-discoverability-and-ux-brea.md&quot;, &quot;.mana/46.2.1-surface-usage-reporting-in-the-tui-commandhelpstar.md&quot;, &quot;.mana/47-rebuild-imp-around-explicit-runtime-boundaries-pro.md&quot;, &quot;.mana/47.1-contracts-and-ownership-boundary-for-mana-imp-rebu.md&quot;, &quot;.mana/47.6-sequence-the-imp-rebuild-as-an-incremental-migrati.md&quot;, &quot;.mana/50-define-cli-first-operator-surface-for-imp-with-tui.md&quot;, &quot;.mana/50.10-implement-guided-cli-parity-flows-for-settings-per.md&quot;, &quot;.mana/50.10.1-implement-terminal-native-imp-settings-flow-for-cl.md&quot;, &quot;.mana/50.10.1.2-let-imp-chat-no-tools-reach-the-shell-without-prov.md&quot;, &quot;.mana/50.10.2-implement-terminal-native-imp-personality-flow-for.md&quot;, &quot;.mana/50.11-implement-first-shell-to-view-handoff-for-sessions.md&quot;, &quot;.mana/50.11.2-align-imp-chat-view-handoff-with-explicit-imp-view.md&quot;, &quot;.mana/50.12-flip-plain-imp-to-imp-chat-after-shell-readiness-g.md&quot;, &quot;.mana/50.13-plan-extraction-of-shared-fullscreen-consumed-runt.md&quot;, &quot;.mana/50.14-specify-the-shared-imp-ui-request-and-runtime-even.md&quot;, &quot;.mana/50.16-follow-on-cli-native-affordance-stack-after-505-se.md&quot;, &quot;.mana/50.16.1-define-stable-human-vs-machine-output-modes-across.md&quot;, &quot;.mana/50.16.2-plan-cli-first-checkpoint-productization-after-out.md&quot;, &quot;.mana/50.16.3-plan-visible-cli-first-planning-artifacts-and-exec.md&quot;, &quot;.mana/50.16.4-plan-first-class-approval-policy-layer-for-cli-fir.md&quot;, &quot;.mana/50.16.5-surface-session-browsing-and-session-search-as-fir.md&quot;, &quot;.mana/50.16.5.1-audit-and-reconcile-imp-session-storage-and-sessio.md&quot;, &quot;.mana/50.16.6-plan-detachedbackground-local-execution-after-cli.md&quot;, &quot;.mana/50.17-define-stable-human-vs-machine-output-contracts-fo.md&quot;, &quot;.mana/50.18-define-cli-first-session-browsing-and-sessionsearc.md&quot;, &quot;.mana/50.19-define-stable-imp-human-vs-machine-output-contract.md&quot;, &quot;.mana/50.20-plan-first-cli-first-checkpoint-productization-ove.md&quot;, &quot;.mana/50.21-specify-visible-planning-artifacts-and-checklist-b.md&quot;, &quot;.mana/50.22-specify-the-first-visible-planning-workflow-and-pl.md&quot;, &quot;.mana/50.23-specify-cli-first-approval-policy-and-blocked-stat.md&quot;, &quot;.mana/50.24-define-the-first-cli-first-approval-policy-surface.md&quot;, &quot;.mana/50.25-specify-detachedbackground-local-execution-contrac.md&quot;, &quot;.mana/50.26-define-the-first-local-detachedbackground-executio.md&quot;, &quot;.mana/50.6-design-the-cli-first-interactive-shell-path-for-im.md&quot;, &quot;.mana/50.9-implement-the-first-cli-first-proving-slice-with-e.md&quot;, &quot;.mana/51.6.1-audit-current-mana-core-embedding-surface-against.md&quot;, &quot;.mana/65-root-mana-currently-lists-child-513-but-direct-sho.md&quot;, &quot;.mana/69-imp-cli-no-longer-contains-duplicate-mana-unit-loa.md&quot;, &quot;.mana/73-code-intelligence-outputs-are-transient-by-default.md&quot;, &quot;.mana/81-design-imp-native-delegation-tool-around-imp-run-a.md&quot;, &quot;.mana/81.10-define-codemap-backed-context-seam-for-imp-run-and.md&quot;, &quot;.mana/82-assess-gpt-54-pro-support-through-openai-chatgpt-o.md&quot;, &quot;.mana/82.2-add-gpt-54-pro-to-imp-model-registry-only-after-oa.md&quot;, &quot;.mana/83-harden-imp-tui-text-box-cursor-and-bounds-handling.md&quot;, &quot;.mana/RULES.md&quot;, &quot;.mana/archive/2026/03/.2-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/16-imp-core-hardening-production-ready-agent-engine.md&quot;, &quot;.mana/archive/2026/03/16.1-wire-config-agent-agentbuilder-thresholds-hooks-re.md&quot;, &quot;.mana/archive/2026/03/16.2-tool-argument-validation-json-schema-before-execut.md&quot;, &quot;.mana/archive/2026/03/16.3-llm-retry-with-exponential-backoff-and-jitter.md&quot;, &quot;.mana/archive/2026/03/16.4-loop-detection-prevent-infinite-tool-call-retry-lo.md&quot;, &quot;.mana/archive/2026/03/16.5-file-not-found-suggestions-with-levenshtein-rankin.md&quot;, &quot;.mana/archive/2026/03/16.6-auto-resume-after-compaction-re-queue-original-pro.md&quot;, &quot;.mana/archive/2026/03/16.7-file-read-tracking-and-staleness-detection.md&quot;, &quot;.mana/archive/2026/03/16.8-file-version-history-pre-edit-snapshots-for-rollba.md&quot;, &quot;.mana/archive/2026/03/17-imp-efficiency-enable-prompt-caching.md&quot;, &quot;.mana/archive/2026/03/19-imp-efficiency-in-session-file-content-cache.md&quot;, &quot;.mana/archive/2026/03/20-imp-efficiency-parallelize-grep-block-search-with.md&quot;, &quot;.mana/archive/2026/03/229-imp-rust-coding-agent-engine.md&quot;, &quot;.mana/archive/2026/03/229.1-workspace-setup-imp-llm-types.md&quot;, &quot;.mana/archive/2026/03/229.10-imp-llm-anthropic-oauth.md&quot;, &quot;.mana/archive/2026/03/229.11-imp-core-hook-system.md&quot;, &quot;.mana/archive/2026/03/229.12-imp-core-tree-sitter-tools-probesearch-probeextrac.md&quot;, &quot;.mana/archive/2026/03/229.13-imp-core-config-resource-discovery.md&quot;, &quot;.mana/archive/2026/03/229.14-imp-core-system-prompt-assembly.md&quot;, &quot;.mana/archive/2026/03/229.15-imp-lua-lua-extension-runtime.md&quot;, &quot;.mana/archive/2026/03/229.16-imp-core-shell-tool-loader.md&quot;, &quot;.mana/archive/2026/03/229.17-imp-tui-ratatui-interactive-mode.md&quot;, &quot;.mana/archive/2026/03/229.18-imp-cli-binary-entry-point.md&quot;, &quot;.mana/archive/2026/03/229.2-imp-llm-anthropic-provider.md&quot;, &quot;.mana/archive/2026/03/229.3-imp-core-tool-trait-file-tools-read-write-edit-mul.md&quot;, &quot;.mana/archive/2026/03/229.4-imp-core-bash-grep-find-tools.md&quot;, &quot;.mana/archive/2026/03/229.5-imp-core-ask-diff-tools.md&quot;, &quot;.mana/archive/2026/03/229.6-imp-core-agent-loop.md&quot;, &quot;.mana/archive/2026/03/229.7-imp-core-session-manager.md&quot;, &quot;.mana/archive/2026/03/229.8-imp-core-context-management-observation-masking-co.md&quot;, &quot;.mana/archive/2026/03/229.9-imp-llm-openai-google-providers.md&quot;, &quot;.mana/archive/2026/03/23-learning-loop-agent-curated-memory-skill-managemen.md&quot;, &quot;.mana/archive/2026/03/23.1-system-prompt-layer-6-wire-memory-into-prompt-asse.md&quot;, &quot;.mana/archive/2026/03/23.2-memory-store-and-memory-tool.md&quot;, &quot;.mana/archive/2026/03/23.3-skill-manage-tool-agent-creates-patches-and-delete.md&quot;, &quot;.mana/archive/2026/03/23.4-learning-nudges-system-prompt-text-and-onagentend.md&quot;, &quot;.mana/archive/2026/03/23.5-session-index-with-fts5-full-text-search.md&quot;, &quot;.mana/archive/2026/03/23.6-session-search-tool.md&quot;, &quot;.mana/archive/2026/03/24-tui-ux-overhaul-information-density-summaries-inte.md&quot;, &quot;.mana/archive/2026/03/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/03/24.2-progress-indicator-in-status-bar-during-streaming.md&quot;, &quot;.mana/archive/2026/03/24.3-per-tool-call-expandcollapse-and-auto-expand-error.md&quot;, &quot;.mana/archive/2026/03/24.4-turn-end-summary-with-file-change-tracking.md&quot;, &quot;.mana/archive/2026/03/24.5-visual-separation-of-tool-activity-from-assistant.md&quot;, &quot;.mana/archive/2026/03/24.6-editor-polish-placeholder-model-indicator-keybindi.md&quot;, &quot;.mana/archive/2026/03/24.7-fix-context-window-tracking-use-actual-conversatio.md&quot;, &quot;.mana/archive/2026/03/24.8-approval-flow-wire-userinterface-for-tool-confirma.md&quot;, &quot;.mana/archive/2026/03/25-multi-provider-llm-support-with-data-driven-welcom.md&quot;, &quot;.mana/archive/2026/03/25.1-provider-metadata-registry-auth-generalization.md&quot;, &quot;.mana/archive/2026/03/25.2-openai-compatible-chat-completions-provider.md&quot;, &quot;.mana/archive/2026/03/25.3-add-builtin-models-for-new-providers.md&quot;, &quot;.mana/archive/2026/03/25.4-data-driven-welcome-flow.md&quot;, &quot;.mana/archive/2026/03/25.5-generalize-cli-login-for-all-providers.md&quot;, &quot;.mana/archive/2026/03/26-fix-imp-tui-compile-errors-around-toolcallorder-re.md&quot;, &quot;.mana/archive/2026/03/27.1-imp-core-mana-tool-add-native-orchestration-action.md&quot;, &quot;.mana/archive/2026/03/31-add-configurable-engineering-guardrails-to-imp.md&quot;, &quot;.mana/archive/2026/03/37.1-design-canonical-usage-schema-and-aggregation-help.md&quot;, &quot;.mana/archive/2026/03/37.2-persist-canonical-usage-entries-in-imp-core-sessio.md&quot;, &quot;.mana/archive/2026/03/37.3-unify-usage-persistence-across-imp-execution-paths.md&quot;, &quot;.mana/archive/2026/03/37.4-add-imp-usage-reporting-commands-and-export.md&quot;, &quot;.mana/archive/2026/04/.10-define-clean-mana-vs-imp-boundary-and-memory-conso.md&quot;, &quot;.mana/archive/2026/04/.10.1-define-imp-memory-layer-architecture-and-mana-ownership-boundaries.md&quot;, &quot;.mana/archive/2026/04/.10.2-design-a-mana-wiki-schema-and-knowledge-maintenance-workflow.md&quot;, &quot;.mana/archive/2026/04/.10.3-strengthen-mana-first-prompt-doctrine-for-durable-planning.md&quot;, &quot;.mana/archive/2026/04/.10.4-design-mana-aware-runtime-context-read-path-for-prompt-assembly.md&quot;, &quot;.mana/archive/2026/04/.10.5-design-inline-mana-state-and-knowledge-surfaces-for-imp-runtime.md&quot;, &quot;.mana/archive/2026/04/24.1-turn-activity-tracker-foundation-for-progress-and.md&quot;, &quot;.mana/archive/2026/04/266.4.3.4-fix-stale-secret-metadata-and-missing-keychain-dia.md&quot;, &quot;.mana/archive/2026/04/27.4-imp-promptingtool-guidance-prefer-native-mana-tool.md&quot;, &quot;.mana/archive/2026/04/272-add-kimi-model-compatibility-and-fix-ctrll-model-p.md&quot;, &quot;.mana/archive/2026/04/274-audit-and-simplify-imp-core-config-module.md&quot;, &quot;.mana/archive/2026/04/28-surface-built-in-features-already-implemented-in-i.md&quot;, &quot;.mana/archive/2026/04/28.1.1-specify-the-strengthened-imp-run-worker-contract-a.md&quot;, &quot;.mana/archive/2026/04/28.1.2-implement-reusable-imp-side-mana-unit-worker-runti.md&quot;, &quot;.mana/archive/2026/04/28.1.3-integrate-mana-run-with-the-strengthened-imp-run-w.md&quot;, &quot;.mana/archive/2026/04/28.1.5-fix-native-imp-delegate-worker-defaults-for-openai.md&quot;, &quot;.mana/archive/2026/04/28.1.5-make-imps-native-mana-tool-the-clear-first-class-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.2-fix-direct-imp-run-codexopenai-worker-request-defa.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.2-extract-shared-model-first-runtime-connection-reso.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.3-refactor-headless-worker-auth-to-normalize-empty-o.md&quot;, &quot;.mana/archive/2026/04/28.1.5.3.4-clarify-imp-to-imp-tool-vocabulary-and-align-docs.md&quot;, &quot;.mana/archive/2026/04/29.3-add-recent-session-previews-to-the-imp-startup-pan.md&quot;, &quot;.mana/archive/2026/04/29.4-add-context-aware-quickstart-guidance-and-health-s.md&quot;, &quot;.mana/archive/2026/04/29.6.1-implement-native-mana-scope-targeting-in-imp-tool.md&quot;, &quot;.mana/archive/2026/04/29.6.2-implement-safe-partial-mana-update-semantics-in-im.md&quot;, &quot;.mana/archive/2026/04/29.6.3-implement-append-style-mana-actions-for-conversati.md&quot;, &quot;.mana/archive/2026/04/30-render-compact-widgetstatus-surfaces-already-suppo.md&quot;, &quot;.mana/archive/2026/04/31.1-write-the-engineering-guardrails-design-note-for-i.md&quot;, &quot;.mana/archive/2026/04/32-productize-checkpoints-from-imps-existing-file-sna.md&quot;, &quot;.mana/archive/2026/04/32.1-checkpoint-foundation-shared-filehistory-wiring-an.md&quot;, &quot;.mana/archive/2026/04/32.2-checkpoint-persistence-session-custom-records-plus.md&quot;, &quot;.mana/archive/2026/04/32.3-checkpoint-ux-minimal-slash-command-list-and-resto.md&quot;, &quot;.mana/archive/2026/04/42-per-agent-cached-context-assembly-for-mana-dispatc.md&quot;, &quot;.mana/archive/2026/04/47.1.4-implement-the-first-shared-verifier-and-evidence-r.md&quot;, &quot;.mana/index.yaml.old&quot;, &quot;.mana/migration-conflicts/.3-add-secure-generic-credential-storage-and-lua-secr.md.txt&quot;, &quot;.mana/migration-conflicts/267-fix-native-imp-worker-openai-route-failure-when-sp.md.txt&quot;, &quot;.mana/migration-conflicts/27-native-mana-tool-overhaul-background-runs-lightwei.md.txt&quot;, &quot;.mana/migration-conflicts/270-make-uu-install-support-active-shell-binary-repair.md.txt&quot;, &quot;.mana/migration-conflicts/270.1-make-uu-install-complete-the-active-shell-imp-upgr.md.txt&quot;, &quot;.mana/migration-conflicts/271-harden-spawn-and-mana-tool-termination-so-closespa.md.txt&quot;, &quot;.mana/migration-conflicts/271.1-diagnose-hang-paths-in-imp-spawn-and-mana-closetoo.md.txt&quot;, &quot;.mana/migration-conflicts/273-make-pi-typescript-extensions-importable-into-imp.md.txt&quot;, &quot;.mana/migration-conflicts/275-rethink-imp-tui-tool-call-presentation-and-sidebar.md.txt&quot;, &quot;.mana/migration-conflicts/44-rethink-imp-extensions-as-guest-runtimes-with-opti.md.txt&quot;, &quot;.mana/migration-conflicts/44.1-plan-phased-implementation-of-imp-native-code-inte.md.txt&quot;, &quot;.mana/migration-conflicts/45-explore-ast-backed-symbolic-plan-layer-for-imp.md.txt&quot;, &quot;.mana/migration-conflicts/51-easy-fix-impmana-gaps-triaged-from-repo-scan.md.txt&quot;, &quot;.tmp/imp-run-trial/one-shot-print.txt&quot;, &quot;.vibecheck/vibecheck.db&quot;, &quot;.vibecheck/vibecheck.db-shm&quot;, &quot;.vibecheck/vibecheck.db-wal&quot;, &quot;=&quot;, &quot;AGENTS copy.md&quot;, &quot;art.html&quot;, &quot;art.html.bak&quot;, &quot;art.md&quot;, &quot;art_original.html&quot;, &quot;art_test.txt&quot;, &quot;crates/imp-cli/auth.json&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-tui/src/views/editor.rs&quot;, &quot;crates/imp-tui/src/views/top_bar.rs&quot;, &quot;draft.html&quot;, &quot;evals/dirac-comparison/tasks/DynamicCache.json&quot;, &quot;evals/dirac-comparison/tasks/IOverlayWidget.json&quot;, &quot;evals/dirac-comparison/tasks/addLogging.json&quot;, &quot;evals/dirac-comparison/tasks/datadict.json&quot;, &quot;evals/dirac-comparison/tasks/extensionswb_service.json&quot;, &quot;evals/dirac-comparison/tasks/latency.json&quot;, &quot;evals/dirac-comparison/tasks/sendRequest.json&quot;, &quot;evals/dirac-comparison/tasks/stoppingcriteria.json&quot;, &quot;gen_art.py&quot;, &quot;tmp-find-django.sh&quot;, &quot;tools/imp-fix-signature.sh&quot;], &quot;insertions&quot;: 22, &quot;deletions&quot;: 30014, &quot;risk_score&quot;: 5, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;very high churn (30036 lines)&quot;]}, {&quot;sha&quot;: &quot;31e1a04ab84b95d91e150b6600bf0f5e4523c3cd&quot;, &quot;short&quot;: &quot;31e1a04&quot;, &quot;subject&quot;: &quot;Build workflow runtime foundations&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T15:31:11-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-cli/src/lib.rs&quot;, &quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/agent/loop_state.rs&quot;, &quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/agent/run_loop.rs&quot;, &quot;crates/imp-core/src/agent/tool_execution.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/context_prefill.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/imp_session.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_prompt_context.rs&quot;, &quot;crates/imp-core/src/mana_worker.rs&quot;, &quot;crates/imp-core/src/reference_monitor.rs&quot;, &quot;crates/imp-core/src/tools/ask.rs&quot;, &quot;crates/imp-core/src/tools/bash.rs&quot;, &quot;crates/imp-core/src/tools/edit.rs&quot;, &quot;crates/imp-core/src/tools/extend.rs&quot;, &quot;crates/imp-core/src/tools/git.rs&quot;, &quot;crates/imp-core/src/tools/mana.rs&quot;, &quot;crates/imp-core/src/tools/memory.rs&quot;, &quot;crates/imp-core/src/tools/mod.rs&quot;, &quot;crates/imp-core/src/tools/multi_edit.rs&quot;, &quot;crates/imp-core/src/tools/read.rs&quot;, &quot;crates/imp-core/src/tools/scan/mod.rs&quot;, &quot;crates/imp-core/src/tools/session_search.rs&quot;, &quot;crates/imp-core/src/tools/shell.rs&quot;, &quot;crates/imp-core/src/tools/web/mod.rs&quot;, &quot;crates/imp-core/src/tools/worktree.rs&quot;, &quot;crates/imp-core/src/tools/write.rs&quot;, &quot;crates/imp-core/src/trust.rs&quot;, &quot;crates/imp-core/src/typescript_extensions/mod.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;crates/imp-core/src/workflow/verification.rs&quot;, &quot;crates/imp-core/src/workflow/verification_runner.rs&quot;, &quot;crates/imp-lua/src/sandbox.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/turn_tracker.rs&quot;, &quot;crates/imp-tui/src/views/command_palette.rs&quot;, &quot;docs/autonomy-modes.md&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;, &quot;docs/reference-monitor-policy.md&quot;, &quot;docs/trace-and-evidence-format.md&quot;, &quot;docs/trust-labels-and-provenance.md&quot;, &quot;docs/verification-gates.md&quot;, &quot;docs/worktree-auto.md&quot;], &quot;insertions&quot;: 8086, &quot;deletions&quot;: 108, &quot;risk_score&quot;: 45, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/mana_worker&quot;, &quot;touches crates/imp-core/src/reference_monitor&quot;, &quot;touches crates/imp-core/src/tools/mana&quot;, &quot;touches crates/imp-core/src/workflow&quot;, &quot;touches crates/imp-tui/src/app&quot;, &quot;very high churn (8194 lines)&quot;]}, {&quot;sha&quot;: &quot;424795c9063683de1bce9fee5866bf69028c3599&quot;, &quot;short&quot;: &quot;424795c&quot;, &quot;subject&quot;: &quot;Trace TUI agent startup phases&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T12:26:38-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 72, &quot;deletions&quot;: 15, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;d89bafd65ac2f18f6d453f0be3a57df0e0b7b8c3&quot;, &quot;short&quot;: &quot;d89bafd&quot;, &quot;subject&quot;: &quot;Keep title spinner active during agent startup&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:56:18-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 42, &quot;deletions&quot;: 4, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;4543ff22420bf6fdb6a4e03055ac370499baa6f0&quot;, &quot;short&quot;: &quot;4543ff2&quot;, &quot;subject&quot;: &quot;Animate chat waiting placeholder each tick&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:28:10-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 30, &quot;deletions&quot;: 7, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;ef9ccdd138fd69da5959be53846f883c64d6f8f8&quot;, &quot;short&quot;: &quot;ef9ccdd&quot;, &quot;subject&quot;: &quot;Start TUI agents off the input path&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T08:57:59-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 214, &quot;deletions&quot;: 154, &quot;risk_score&quot;: 7, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;moderate churn (368 lines)&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;79665f4209ab43760f14f3f635a74434826c069d&quot;, &quot;short&quot;: &quot;79665f4&quot;, &quot;subject&quot;: &quot;Restore faster title spinner cadence&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:13:19-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 8, &quot;deletions&quot;: 8, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;ceb3ef3abdfe6361fbe6daec3b24ce328d52690c&quot;, &quot;short&quot;: &quot;ceb3ef3&quot;, &quot;subject&quot;: &quot;Use clearer title spinner cadence&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T11:11:18-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 14, &quot;deletions&quot;: 14, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;98cbab62f34389479859bde907fc5b78ddf3e537&quot;, &quot;short&quot;: &quot;98cbab6&quot;, &quot;subject&quot;: &quot;Reuse rendered tool click map for inspector&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T10:24:33-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/app.rs&quot;, &quot;crates/imp-tui/src/views/chat.rs&quot;], &quot;insertions&quot;: 67, &quot;deletions&quot;: 16, &quot;risk_score&quot;: 4, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;b6c301ec1dcb1b0519bbc0d74883885f14b63a48&quot;, &quot;short&quot;: &quot;b6c301e&quot;, &quot;subject&quot;: &quot;Use spinner for TUI working title&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-13T10:10:56-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-tui/src/animation.rs&quot;, &quot;crates/imp-tui/src/app.rs&quot;], &quot;insertions&quot;: 18, &quot;deletions&quot;: 18, &quot;risk_score&quot;: 6, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;, &quot;touches crates/imp-tui/src/app&quot;]}, {&quot;sha&quot;: &quot;2b6ef71be3d20f628223b9be70bd28ce55290892&quot;, &quot;short&quot;: &quot;2b6ef71&quot;, &quot;subject&quot;: &quot;Document TUI workflow wireframes&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T12:24:20-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;docs/tui-workflow-wireframes.md&quot;], &quot;insertions&quot;: 753, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 3, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;high churn (753 lines)&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;79b49633d66ee8280af9682c945cab5425a7c428&quot;, &quot;short&quot;: &quot;79b4963&quot;, &quot;subject&quot;: &quot;Add trace and evidence artifacts&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:26:51-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/agent/events.rs&quot;, &quot;crates/imp-core/src/evidence.rs&quot;, &quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/storage.rs&quot;, &quot;crates/imp-core/src/trace.rs&quot;, &quot;docs/trace-and-evidence-format.md&quot;], &quot;insertions&quot;: 1397, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 5, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;high churn (1397 lines)&quot;, &quot;mostly docs/&quot;, &quot;touches crates/imp-core/src/agent&quot;]}, {&quot;sha&quot;: &quot;e2dba93ca9660c2a24a6256e750773de30e67601&quot;, &quot;short&quot;: &quot;e2dba93&quot;, &quot;subject&quot;: &quot;Add mana workflow ledger model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:26:27-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/lib.rs&quot;, &quot;crates/imp-core/src/mana_next/ledger.rs&quot;, &quot;crates/imp-core/src/mana_next/mod.rs&quot;, &quot;docs/mana-next-compatibility-adapter.md&quot;, &quot;docs/mana-next-examples.md&quot;, &quot;docs/mana-next-migration-test-plan.md&quot;, &quot;docs/mana-next-runtime-event-mapping.md&quot;, &quot;docs/mana-next-storage-strategy.md&quot;, &quot;docs/mana-next-ux.md&quot;, &quot;docs/mana-next-workflow-ledger.md&quot;], &quot;insertions&quot;: 2578, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 0, &quot;risk_label&quot;: &quot;low&quot;, &quot;risk_reasons&quot;: [&quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;very high churn (2578 lines)&quot;]}, {&quot;sha&quot;: &quot;c483434eba3b7434ae4c6f8739afbceeef9567e2&quot;, &quot;short&quot;: &quot;c483434&quot;, &quot;subject&quot;: &quot;Add workflow contract model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:25:46-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/lib.rs&quot;], &quot;insertions&quot;: 1, &quot;deletions&quot;: 0, &quot;risk_score&quot;: 2, &quot;risk_label&quot;: &quot;medium&quot;, &quot;risk_reasons&quot;: [&quot;risky subject keyword&quot;]}, {&quot;sha&quot;: &quot;0184de68d2fc157f6127826c7e1743799a19d7df&quot;, &quot;short&quot;: &quot;0184de6&quot;, &quot;subject&quot;: &quot;Add workflow contract model&quot;, &quot;author&quot;: &quot;Asher Bronstein&quot;, &quot;date&quot;: &quot;2026-05-12T10:23:35-07:00&quot;, &quot;side&quot;: &quot;workflow-only&quot;, &quot;parents&quot;: 1, &quot;files&quot;: [&quot;crates/imp-core/src/agent/mod.rs&quot;, &quot;crates/imp-core/src/builder.rs&quot;, &quot;crates/imp-core/src/workflow/contract.rs&quot;, &quot;crates/imp-core/src/workflow/mod.rs&quot;, &quot;docs/imp-next-workflow-runtime.md&quot;], &quot;insertions&quot;: 1252, &quot;deletions&quot;: 2, &quot;risk_score&quot;: 15, &quot;risk_label&quot;: &quot;high&quot;, &quot;risk_reasons&quot;: [&quot;high churn (1254 lines)&quot;, &quot;mostly docs/&quot;, &quot;risky subject keyword&quot;, &quot;touches crates/imp-core/src/agent&quot;, &quot;touches crates/imp-core/src/workflow&quot;]}]</script>
docs/workflows.md:3:imp workflows are local project artifacts for planned, multi-step work. They keep the plan, execution state, checks, prototype results, events, and closeout notes in files under the project.
docs/workflows.md:126:- API-addressable workflows are planned, not shipped.
docs/mana-next-compatibility-adapter.md:1:# mana-next Compatibility Adapter
docs/mana-next-compatibility-adapter.md:4:Parent: mana `394.3` / child `394.3.3`
docs/mana-next-compatibility-adapter.md:8:The compatibility adapter lets imp-next view existing mana units as workflow-ledger records without breaking current mana commands or requiring a migration.
docs/mana-next-compatibility-adapter.md:13:current mana markdown/frontmatter + optional sidecars + imp run artifacts
docs/mana-next-compatibility-adapter.md:23:- existing `.mana/*.md` unit files
docs/mana-next-compatibility-adapter.md:25:- current mana decisions/notes where available
docs/mana-next-compatibility-adapter.md:26:- optional sidecars under `~/.mana/ledger/`
docs/mana-next-compatibility-adapter.md:40:These views are not necessarily new storage records. They are the compatibility interface used by imp workflow runtime, evidence, TUI summaries, and future mana-next commands.
docs/mana-next-compatibility-adapter.md:81:If `mana verify` has a recorded result in future metadata/sidecars, that result should populate status. Without result metadata, status is `pending` or `unknown` depending on context.
docs/mana-next-compatibility-adapter.md:93:The adapter should preserve direction exactly as existing mana commands interpret it.
docs/mana-next-compatibility-adapter.md:103:Current mana decisions map directly to DecisionView.
docs/mana-next-compatibility-adapter.md:138:2. Sidecars under `~/.mana/ledger/` for structured repeated records.
docs/mana-next-compatibility-adapter.md:154:If imp is asked to run mana unit `394.2.1`:
docs/mana-next-compatibility-adapter.md:178:Current `mana close` remains authoritative for old status updates. imp-next closeout should call compatible update APIs rather than bypass mana status semantics.
docs/mana-next-compatibility-adapter.md:209:This API can live in imp-core initially and use current mana tool/CLI/file APIs under the hood.
docs/mana-next-compatibility-adapter.md:213:### Current mana task
docs/mana-next-compatibility-adapter.md:243:### Current mana epic
docs/mana-next-compatibility-adapter.md:268:| user-created mana unit | durable_mana_record |
docs/mana-next-compatibility-adapter.md:272:| old fact with TTL | mana_fact_staleable |
docs/mana-next-compatibility-adapter.md:279:- Should sidecars be addressed by mana unit ID, run ID, or both?
docs/mana-next-compatibility-adapter.md:281:- How should current `mana verify` results be persisted so VerificationView can show status?
docs/index.md:8:- [ACP editor adapter](acp.md) — `imp acp`, ACP stdio handshake/session scaffold, current limitations, and editor launch shape.
docs/role-registry.md:276:tools = ["read", "scan", "edit", "write", "bash", "git", "mana"]
docs/mana-next-storage-strategy.md:1:# mana-next Storage and Artifact Reference Strategy
docs/mana-next-storage-strategy.md:4:Parent: mana `394.3` / child `394.3.2`
docs/mana-next-storage-strategy.md:10:1. Keep existing mana units as markdown files under `~/.mana` as the source of truth for workflow/task/decision/note compatibility.
docs/mana-next-storage-strategy.md:12:3. Store bulky run artifacts in imp run directories, not mana files.
docs/mana-next-storage-strategy.md:15:This preserves current mana behavior while giving imp-next stable references for evidence, verification, workflow contracts, and child runs.
docs/mana-next-storage-strategy.md:19:Current mana uses file-backed markdown units such as:
docs/mana-next-storage-strategy.md:22:~/.mana/394.3.1-specify-mana-next-workflow-ledger-schema-and-compa.md
docs/mana-next-storage-strategy.md:23:~/.mana/394.7.8-accept-user-and-mana-provided-verification-gates.md
docs/mana-next-storage-strategy.md:28:Therefore v1 mana-next must not require:
docs/mana-next-storage-strategy.md:31:- moving units out of `~/.mana`
docs/mana-next-storage-strategy.md:37:### Layer 1: mana unit markdown
docs/mana-next-storage-strategy.md:48:- notes/decisions in current mana format
docs/mana-next-storage-strategy.md:68:This can be added as optional frontmatter later. Old mana ignores unknown fields.
docs/mana-next-storage-strategy.md:77:~/.mana/ledger/
docs/mana-next-storage-strategy.md:158:- Do not inline large outputs in mana records.
docs/mana-next-storage-strategy.md:164:For a workflow backed by an existing mana unit:
docs/mana-next-storage-strategy.md:167:~/.mana/394.3-streamline-mana-into-workflow-and-evidence-ledger.md
docs/mana-next-storage-strategy.md:172:For a workflow not explicitly backed by mana:
docs/mana-next-storage-strategy.md:179:| Run type | mana record? | artifacts? |
docs/mana-next-storage-strategy.md:183:| mana task run | yes, existing unit | yes |
docs/mana-next-storage-strategy.md:209:The command may also remain in the current mana `verify` field for compatibility.
docs/mana-next-storage-strategy.md:215:Do not write large evidence content into `.mana/*.md` frontmatter.
docs/mana-next-storage-strategy.md:237:- `mana list`
docs/mana-next-storage-strategy.md:238:- `mana show`
docs/mana-next-storage-strategy.md:239:- `mana create`
docs/mana-next-storage-strategy.md:240:- `mana update`
docs/mana-next-storage-strategy.md:241:- `mana verify`
docs/mana-next-storage-strategy.md:242:- `mana close`
docs/mana-next-storage-strategy.md:243:- `mana notes_append`
docs/mana-next-storage-strategy.md:244:- `mana decision_*`
docs/mana-next-storage-strategy.md:245:- `mana dep_*`
docs/mana-next-storage-strategy.md:264:- Add optional workflow/evidence frontmatter fields only when current mana preserves unknown fields safely.
docs/mana-next-storage-strategy.md:276:- Old mana can ignore `~/.mana/ledger` sidecars.
docs/mana-next-storage-strategy.md:278:- Run artifacts can remain as historical evidence even if mana-next is disabled.
docs/mana-next-storage-strategy.md:325:- harder to inspect from `mana show`
docs/mana-next-storage-strategy.md:333:~/.mana/
docs/mana-next-storage-strategy.md:334:  394.3-streamline-mana-into-workflow-and-evidence-ledger.md
docs/mana-next-ux.md:1:# mana-next UX and Progressive Disclosure
docs/mana-next-ux.md:4:Parent: mana `394.3` / child `394.3.8`
docs/mana-next-ux.md:8:mana-next should feel invisible for routine TUI work and invaluable when work becomes durable, blocked, verified, resumed, delegated, or reviewed.
docs/mana-next-ux.md:10:The user should not need to understand mana to ask imp to fix a small issue. But when the task matters, mana should provide the durable workflow ledger: status, blockers, decisions, verification, evidence, and closeout.
docs/mana-next-ux.md:15:2. **Progressive disclosure.** Show workflow/mana details only when useful.
docs/mana-next-ux.md:16:3. **Automatic bookkeeping.** The agent writes durable summaries/evidence refs; users should not manually update mana after every step.
docs/mana-next-ux.md:17:4. **No transcript spam.** mana stores workflow summaries and artifact refs, not raw logs.
docs/mana-next-ux.md:18:5. **Recovery-oriented.** A user should be able to resume, inspect blockers, and find evidence from mana.
docs/mana-next-ux.md:30:- no visible mana ceremony
docs/mana-next-ux.md:51:- mana stores durable summary/evidence refs
docs/mana-next-ux.md:61:- mana unit is the workflow/task anchor
docs/mana-next-ux.md:63:- verification and evidence refs attach to the mana ledger
docs/mana-next-ux.md:84:mana list
docs/mana-next-ux.md:85:mana show 394.2
docs/mana-next-ux.md:86:mana verify 394.2.1
docs/mana-next-ux.md:87:mana close 394.2.1
docs/mana-next-ux.md:100:These should read the mana ledger, not replace it.
docs/mana-next-ux.md:135:## What mana records automatically
docs/mana-next-ux.md:160:- mana workflow/task show
docs/mana-next-ux.md:172:mana stores refs and summaries.
docs/mana-next-ux.md:190:## Compatibility with current mana
docs/mana-next-ux.md:192:Current mana remains valid:
docs/mana-next-ux.md:204:### Do I need to create a mana workflow before using imp?
docs/mana-next-ux.md:206:No. Routine TUI use should work without manual mana setup.
docs/mana-next-ux.md:208:### When does mana matter?
docs/mana-next-ux.md:212:### Does mana store every agent message?
docs/mana-next-ux.md:214:No. Raw traces live in run artifacts. mana stores durable summaries and references.
docs/mana-next-ux.md:216:### Can I still use current mana commands?
docs/mana-next-ux.md:218:Yes. mana-next is designed as a compatibility layer first.
docs/mana-next-ux.md:220:### Is this a project-management system?
docs/worktree-auto.md:39:- tmux/team orchestration
docs/worktree-auto.md:57:`mana_core::worktree::detect_worktree(cwd)` in list output to distinguish a
docs/worktree-auto.md:221:Evidence/mana should record:
docs/worktree-auto.md:337:- 394.9.7: trace/evidence/mana metadata refs
docs/verification-gates.md:170:- mana task verify command
docs/verification-gates.md:261:5. Accept user/mana-provided gate declarations.
docs/verification-gates.md:268:from the user, mana task, workflow contract, or trusted config. Inference is a
docs/verification-gates.md:318:7. There is no explicit user/mana/workflow gate that already covers the same
docs/verification-gates.md:357:#### JavaScript/TypeScript
docs/verification-gates.md:393:Explicit user/mana/workflow gates take precedence over inferred gates. Inference
docs/verification-gates.md:567:translate mana-provided verification into `VerificationGate` records with source
docs/verification-gates.md:568:`ManaTask`. A mana verify gate should behave like any other required gate:
docs/verification-gates.md:575:When closing mana work, store artifact refs or evidence summaries rather than
docs/verification-gates.md:576:inlining full logs. The durable mana record should point to the run evidence and
docs/workflow-profiles.md:74:Workflow profiles are backend-neutral. They should use imp-native-ready concepts such as plan, task, evidence, decision, verification, artifact, and closeout. Current mana compatibility should stay behind adapters; normal users should not need to understand mana terminology.
docs/mana-next-workflow-ledger.md:1:# mana-next Workflow Ledger Schema
docs/mana-next-workflow-ledger.md:4:Parent: mana `394.3` / child `394.3.1`
docs/mana-next-workflow-ledger.md:8:mana-next is the durable workflow ledger for imp-next. It records the stable state and reviewable evidence of agent work without becoming a project-management UI or a transcript dump.
docs/mana-next-workflow-ledger.md:19:Existing mana units remain compatible. This document defines how the new vocabulary maps onto the current file-backed mana concepts: epic, task, fact, decision, notes, dependencies, verify commands, acceptance criteria, and artifacts.
docs/mana-next-workflow-ledger.md:24:- No removal of existing mana primitives.
docs/mana-next-workflow-ledger.md:26:- No transcript storage in mana records.
docs/mana-next-workflow-ledger.md:32:1. **Workflow ledger, not Jira.** mana records execution truth and evidence, not a noisy planning bureaucracy.
docs/mana-next-workflow-ledger.md:33:2. **Summaries in mana, bulk in artifacts.** Raw traces, logs, diffs, and transcripts live under imp run artifacts; mana stores refs and durable summaries.
docs/mana-next-workflow-ledger.md:34:3. **Compatibility first.** Existing `mana list/show/create/update/verify/close/decision/notes/deps` should continue to work.
docs/mana-next-workflow-ledger.md:79:- `planned`
docs/mana-next-workflow-ledger.md:92:| mana-next Workflow | Current mana |
docs/mana-next-workflow-ledger.md:104:A Task is a decomposed execution unit within a Workflow. Current mana `task` maps naturally here.
docs/mana-next-workflow-ledger.md:129:| mana-next Task | Current mana |
docs/mana-next-workflow-ledger.md:163:| mana-next Decision | Current mana |
docs/mana-next-workflow-ledger.md:204:| mana-next Verification | Current mana |
docs/mana-next-workflow-ledger.md:207:| status/result | current `mana verify` outcome plus notes/logs |
docs/mana-next-workflow-ledger.md:246:| mana-next Evidence | Current mana |
docs/mana-next-workflow-ledger.md:272:| mana-next Note | Current mana |
docs/mana-next-workflow-ledger.md:306:  └─ may be summarized in mana
docs/mana-next-workflow-ledger.md:371:Existing mana compatibility can model child runs as child tasks initially.
docs/mana-next-workflow-ledger.md:375:1. Existing file-backed mana units under `~/.mana` must remain readable.
docs/mana-next-workflow-ledger.md:385:<thead><tr><th>mana-next</th><th>Current mana</th><th>Compatibility behavior</th></tr></thead>
docs/mana-next-workflow-ledger.md:401:- Should workflow IDs always equal mana unit IDs, or should imp run workflows have separate run IDs linked to mana units?
docs/mana-next-workflow-ledger.md:402:- How much workflow metadata belongs in `~/.mana` versus repo `.imp/runs` artifacts?
docs/trace-and-evidence-format.md:4:Parent: mana `394.4` / child `394.4.1`
docs/trace-and-evidence-format.md:81:| `workflow_id` | mana/workflow contract id when available |
docs/trace-and-evidence-format.md:161:<tr><td><code>TurnEnd { index, message, mana_review }</code></td><td><code>turn.end</code></td><td>assistant message summary, mana review summary</td><td>Do not inline huge message content in future if artifact exists.</td></tr>
docs/trace-and-evidence-format.md:313:- workflow id / mana id when present
docs/trace-and-evidence-format.md:408:| mana refs | mana workflow ledger adapter |
docs/trace-and-evidence-format.md:492:- Experimental events can use an `experimental.` prefix if needed.
docs/trace-and-evidence-format.md:499:- No mana ledger write path for evidence refs yet.
docs/trace-and-evidence-format.md:500:- No GUI/TUI redesign beyond compact evidence path surfacing.
docs/autonomy-modes.md:125:- Existing bash-equivalent mana blocking remains in force.
docs/autonomy-modes.md:218:constraints, repeated-call protection, bash-equivalent mana blocking, schema
docs/child-workflow-delegation.md:4:Audience: imp maintainers, mana maintainers, TUI/runtime implementers
docs/child-workflow-delegation.md:10:by OMO-style parallel agent work, but the imp core abstraction is workflow + mana
docs/child-workflow-delegation.md:11:ledger + runtime events, not tmux panes or a team chat metaphor.
docs/child-workflow-delegation.md:19:General worker farms, autonomous team mode, and OMO-style full team
docs/child-workflow-delegation.md:29:5. Persist lifecycle state in mana and run artifacts.
docs/child-workflow-delegation.md:30:6. Emit runtime events and snapshots that TUI/GUI can render consistently.
docs/child-workflow-delegation.md:37:- No OMO/OMX-style always-on team simulation.
docs/child-workflow-delegation.md:43:- Full team orchestration is future work.
docs/child-workflow-delegation.md:65:- parent mana unit ref when available
docs/child-workflow-delegation.md:80:    pub parent_mana_unit_ref: Option<String>,
docs/child-workflow-delegation.md:127:Do not create noisy mana tasks for every trivial child unless the child needs
docs/child-workflow-delegation.md:289:- mana notes/facts when durable
docs/child-workflow-delegation.md:355:Events should be stable enough for TUI/GUI consumption:
docs/child-workflow-delegation.md:424:- mana ledger adapters for child refs, child task records, and evidence records
docs/child-workflow-delegation.md:432:- full parallel child scheduling/resource management
docs/child-workflow-delegation.md:435:- OMO-style full team orchestration
docs/child-workflow-delegation.md:442:4. Sequential child workflow manager for verifier/reviewer/researcher.
docs/child-workflow-delegation.md:458:- mana ledger: durable truth
docs/child-workflow-delegation.md:461:This keeps child delegation usable from CLI, TUI, GUI, CI, and mana without
docs/extensions-lua.md:67:Lua is the current shipped extension path. TypeScript extension support exists in repository code paths but should not be documented as the stable shipped extension system.
docs/design/droid-mission-mode-vs-imp-work-plan.md:9:`imp-work` already has many of the right primitives for a Droid Mission-style system, but the existing `run` surface is still **mana compatibility**, not native imp-work orchestration.
docs/design/droid-mission-mode-vs-imp-work-plan.md:15:- `HeadlessManaArgs` requires `unit_id` and `mana_dir`, confirming `imp run` currently targets mana units, not native work tasks/epics.
docs/design/droid-mission-mode-vs-imp-work-plan.md:27:The correct direction is not to invent `run` from scratch. It is to **promote the existing imp-work scheduler/run primitives into a real native `work run` / Work Control product surface, then migrate `imp run` from mana compatibility to native work orchestration once parity is proven.**
docs/design/droid-mission-mode-vs-imp-work-plan.md:243:There is already an `imp run`, but it is hidden mana compatibility.
docs/design/droid-mission-mode-vs-imp-work-plan.md:249:3. Keep hidden `imp run` alias for mana during transition.
docs/design/droid-mission-mode-vs-imp-work-plan.md:258:Goal: avoid breaking existing mana workflows.
docs/design/droid-mission-mode-vs-imp-work-plan.md:262:- Keep current hidden `imp run <mana-id>` behavior for now.
docs/design/droid-mission-mode-vs-imp-work-plan.md:356:- `wave_planned`
docs/design/droid-mission-mode-vs-imp-work-plan.md:546:This should be simple, inspectable, local-first — not project-management bloat.
docs/design/droid-mission-mode-vs-imp-work-plan.md:682:- Does not disturb hidden mana `imp run`.
docs/typescript-extension-bridge.md:1:# TypeScript Extension Bridge
docs/typescript-extension-bridge.md:5:Add TypeScript extension tools without moving authority out of Rust. Extensions
docs/typescript-extension-bridge.md:10:Non-goal: no in-process JavaScript or TypeScript runtime embedded in imp. Rust
docs/typescript-extension-bridge.md:29:  - wraps tools as `TypeScriptExtensionTool`
docs/typescript-extension-bridge.md:43:TypeScript should share the same principles but use a stricter process boundary:
docs/typescript-extension-bridge.md:89:      "description": "Echo text through a TypeScript extension.",
docs/typescript-extension-bridge.md:339:- `invalid TypeScript extension manifest`: schema validation failed; check
docs/typescript-extension-bridge.md:393:## Compatibility with current TypeScriptExtensionToolStatus
docs/typescript-extension-bridge.md:395:`TypeScriptExtensionToolStatus` should grow from compatibility-only state into a
docs/typescript-extension-bridge.md:406:Existing `TypeScriptExtensionCompatibility` can remain for Pi/Bun compatibility
docs/typescript-extension-bridge.md:470:- No custom TS-rendered UI components in the TUI/GUI.
docs/design/imp-work-mana-removal-ledger.md:1:# imp-work mana removal ledger
docs/design/imp-work-mana-removal-ledger.md:3:This ledger records the evidence required before removing mana runtime dependencies from imp.
docs/design/imp-work-mana-removal-ledger.md:7:Do **not** remove mana code in this patch. The parity foundation is now substantially implemented, but the final cutover is destructive and needs explicit approval after review of the ledger and committed migration work.
docs/design/imp-work-mana-removal-ledger.md:13:| `.mana` -> `.imp/work` migration imports active units | `crates/imp-work/src/mana_shadow.rs`; `cargo test -p imp-work mana_migration -- --nocapture` | satisfied for fixture coverage |
docs/design/imp-work-mana-removal-ledger.md:14:| Archived mana units import with history refs | `ManaHistoryRef`, `archived_units`, archive fixture test | satisfied for fixture coverage |
docs/design/imp-work-mana-removal-ledger.md:24:| `.mana` support narrowed to import-only/legacy | Not yet implemented | open before destructive removal |
docs/design/imp-work-mana-removal-ledger.md:25:| Removal of mana tool/mana_worker/mana-core dependency approved | Not yet approved | blocked on explicit approval |
docs/design/imp-work-mana-removal-ledger.md:30:2. Run a real-project `.mana` dry-run import and write-mode import into `.imp/work` backup scope.
docs/design/imp-work-mana-removal-ledger.md:33:5. Mark `.mana` support import-only/legacy.
docs/design/imp-work-mana-removal-ledger.md:35:   - `crates/imp-core/src/tools/mana.rs`
docs/design/imp-work-mana-removal-ledger.md:36:   - `crates/imp-core/src/mana_worker.rs`
docs/design/imp-work-mana-removal-ledger.md:37:   - `mana-core` dependency from imp crates
docs/design/imp-work-mana-removal-ledger.md:38:   - mana-specific agent-loop workflow progress affordances
docs/design/imp-work-mana-removal-ledger.md:43:The destructive removal is intentionally blocked until a real-project migration and routing cutover are reviewed. Fixture parity is strong enough to proceed to cutover preparation, but not enough to delete mana runtime paths safely without approval.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:3:This audit resolves mana unit `264.9`, a follow-up from the broader storage topology audit. It determines whether prompt-template files and TOML-defined shell-tool roots are active production storage surfaces or should be treated as experimental/unwired before the storage contract preserves them.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:63:Prompt templates are **defined but unwired/experimental** in the current shipped path.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:74:But document status as **experimental/unwired** and avoid promising runtime discovery semantics beyond the helper itself.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:124:TOML shell tools are **implemented as a loader and executable tool type, but unwired/experimental** in the current shipped path.
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:130:Keep roots reserved only as experimental candidate roots:
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:150:- prompts and shell tools should be listed as **reserved/experimental surfaces**, not core active surfaces;
docs/rebuild/imp-prompt-shell-tool-storage-wiring-audit.md:173:Both prompt templates and TOML shell tools are source-defined and tested, but neither has a production call site in the inspected shipped paths. Treat them as experimental/reserved storage surfaces in the topology. Do not migrate or promise them as active runtime behavior until separate product and policy work wires them deliberately.
docs/design/droid-gap-map-and-imp-roadmap.md:10:Droid's advantage is mostly packaging: Missions, Custom Droids, MCP, hooks, review workflows, IDE surfaces, and team integrations are presented as coherent product features.
docs/design/droid-gap-map-and-imp-roadmap.md:19:- Do not reintroduce mana terminology. New workflow surfaces should use native `imp-work` terms.
docs/design/droid-gap-map-and-imp-roadmap.md:20:- Do not turn every workflow into project management UI. Keep chat and terminal flow primary.
docs/design/droid-gap-map-and-imp-roadmap.md:21:- Do not build SaaS/team surfaces before local runtime/product quality is strong.
docs/design/droid-gap-map-and-imp-roadmap.md:94:imp is moving from mana to native imp-work. imp-work already has tasks, epics, prototypes, context packs, runs, attempts, leases, path locks, checks, outcomes, memory, and structured scheduler state.
docs/design/droid-gap-map-and-imp-roadmap.md:128:- planned tasks and dependencies
docs/design/droid-gap-map-and-imp-roadmap.md:153:Key imp distinction: Droid Missions feel like multi-agent project management. imp Work Control should feel like inspectable local execution with proof.
docs/design/droid-gap-map-and-imp-roadmap.md:155:## 3. MCP support
docs/design/droid-gap-map-and-imp-roadmap.md:159:Droid has first-class MCP: `/mcp`, `droid mcp add/remove`, HTTP and stdio transports, OAuth, registry, user/project config, disabled tools, and enterprise allowlists.
docs/design/droid-gap-map-and-imp-roadmap.md:163:imp has strong native tools and extensibility, but MCP is not currently a comparable product surface in the inspected docs.
docs/design/droid-gap-map-and-imp-roadmap.md:167:High. MCP has become an expected integration layer.
docs/design/droid-gap-map-and-imp-roadmap.md:190:- `/mcp` manager
docs/design/droid-gap-map-and-imp-roadmap.md:199:- MCP tools become normal tools behind the reference monitor
docs/design/droid-gap-map-and-imp-roadmap.md:202:- MCP calls should appear in traces and evidence like native tool calls
docs/design/droid-gap-map-and-imp-roadmap.md:207:1. stdio MCP
docs/design/droid-gap-map-and-imp-roadmap.md:208:2. HTTP MCP
docs/design/droid-gap-map-and-imp-roadmap.md:211:5. TUI manager
docs/design/droid-gap-map-and-imp-roadmap.md:325:- MCP `linear.read_issue` is allowed, but `linear.update_issue` requires approval
docs/design/droid-gap-map-and-imp-roadmap.md:343:- restrict which MCP servers can access credentials
docs/design/droid-gap-map-and-imp-roadmap.md:386:## 7. ACP support
docs/design/droid-gap-map-and-imp-roadmap.md:390:Droid supports ACP integrations for JetBrains and Zed.
docs/design/droid-gap-map-and-imp-roadmap.md:394:imp is terminal-first and exposes a Rust SDK direction, but ACP is not a documented product surface.
docs/design/droid-gap-map-and-imp-roadmap.md:398:Medium. Important for editor adoption, but lower priority than custom agents/MCP/review if terminal quality is the current focus.
docs/design/droid-gap-map-and-imp-roadmap.md:402:Implement ACP as a thin host adapter over imp-core, not as a separate agent brain.
docs/design/droid-gap-map-and-imp-roadmap.md:411:- use same policy, hooks, agents, skills, MCP, and imp-work runtime as terminal imp
docs/design/droid-gap-map-and-imp-roadmap.md:415:1. document ACP protocol requirements and event mapping
docs/design/droid-gap-map-and-imp-roadmap.md:486:- imp's differentiator is runtime quality, not a closed hosted network effect yet.
docs/design/droid-gap-map-and-imp-roadmap.md:496:1. Open source core now; paid hosted/team features later.
docs/design/droid-gap-map-and-imp-roadmap.md:498:3. Free local CLI; paid Pro when polish and hosted/team features exist.
docs/design/droid-gap-map-and-imp-roadmap.md:532:- hosted sync for imp-work and sessions
docs/design/droid-gap-map-and-imp-roadmap.md:533:- team policy/admin
docs/design/droid-gap-map-and-imp-roadmap.md:534:- Slack/GitHub/Linear hosted adapters
docs/design/droid-gap-map-and-imp-roadmap.md:535:- managed remote workers
docs/design/droid-gap-map-and-imp-roadmap.md:546:3. MCP stdio/HTTP foundation
docs/design/droid-gap-map-and-imp-roadmap.md:549:6. ACP adapter
docs/rebuild/imp-bounded-subagent-orchestration.md:3:Status: target design for mana unit 365.9.
docs/rebuild/imp-bounded-subagent-orchestration.md:11:mana or another harness owns **durable work orchestration across runs**.
docs/rebuild/imp-bounded-subagent-orchestration.md:16:- If child work must survive the parent run, appear on a board, be scheduled independently, hold a lease, or own close/fail/review lifecycle, it belongs in mana or another harness.
docs/rebuild/imp-bounded-subagent-orchestration.md:45:A durable worker is different from a bounded subagent. Durable workers are created and tracked by mana or another host.
docs/rebuild/imp-bounded-subagent-orchestration.md:83:Those belong to mana or another harness.
docs/rebuild/imp-bounded-subagent-orchestration.md:105:Bounded subagents should emit normalized runtime events so CLI, TUI, RPC, and a host GUI can observe them.
docs/rebuild/imp-bounded-subagent-orchestration.md:171:A host such as mana may promote a bounded subagent into durable work when the work becomes too large or needs independent lifecycle.
docs/rebuild/imp-bounded-subagent-orchestration.md:173:Promotion should be explicit. The parent imp run may request promotion, but mana owns the durable record after promotion.
docs/rebuild/imp-bounded-subagent-orchestration.md:184:After promotion, imp may continue as a worker assigned by mana, but the work graph is no longer imp-owned.
docs/rebuild/imp-bounded-subagent-orchestration.md:198:   - mana root bootstrap;
docs/rebuild/imp-bounded-subagent-orchestration.md:199:   - child mana unit creation;
docs/rebuild/imp-bounded-subagent-orchestration.md:200:   - mana graph closeout;
docs/rebuild/imp-bounded-subagent-orchestration.md:206:1. Split `WorkflowRuntimeLayer` internally into recipe/runtime support and mana-work-graph compatibility modules without behavior changes.
docs/rebuild/imp-bounded-subagent-orchestration.md:208:3. Add a no-op/default recipe layer for standalone imp runs and make mana/work-graph compatibility explicitly enabled by host/builder configuration.
docs/rebuild/imp-bounded-subagent-orchestration.md:219:- imp remains useful without mana or imp-work enabled.
docs/design/imp-workflow-responsibility-boundaries.md:3:This note records the current consolidation boundary between imp-native workflows, mana work, prototype experiments, workflow profiles, and child/subagent infrastructure.
docs/design/imp-workflow-responsibility-boundaries.md:21:Mana remains the durable project graph and existing work-unit runtime. `crates/imp-core/src/tools/mana.rs` owns mana discovery, unit operations, native run orchestration, and transitional compatibility with existing mana workflows.
docs/design/imp-workflow-responsibility-boundaries.md:23:`crates/imp-core/src/mana_worker.rs` is the canonical single-unit mana worker runtime. Workflow-native orchestration should bridge to it when a workflow step is explicitly mana-unit-backed, rather than duplicating mana assignment loading.
docs/design/imp-workflow-responsibility-boundaries.md:37:### Transitional mana compatibility
docs/design/imp-workflow-responsibility-boundaries.md:39:`crates/imp-core/src/agent/workflow_integration/mana_compat.rs` infers durable workflow progress from mana/work tool result shapes. Treat this as migration glue. As workflow-native checks/events mature, this inference layer should shrink.
docs/design/imp-workflow-responsibility-boundaries.md:44:2. Keep mana as the durable graph and canonical mana-unit worker substrate.
docs/design/imp-workflow-responsibility-boundaries.md:47:5. Bridge workflow steps to mana/prototype/child-run evidence explicitly instead of merging all systems into one tool.
docs/design/imp-workflow-responsibility-boundaries.md:53:- Add explicit workflow-native progress events to replace parts of `mana_compat` inference.
docs/design/imp-workflow-responsibility-boundaries.md:55:- Add mana-backed workflow step references when a workflow step corresponds to a mana unit.
docs/design/imp-semantic-write-execution-contract.md:3:This contract specifies runtime sequencing for write-oriented semantic actions in imp. It was reconstructed from mana unit `44.1.5.5` because the originally referenced source docs were not present in this worktree.
docs/design/imp-semantic-write-execution-contract.md:17:3. **Freshness check before preview**
docs/design/imp-semantic-write-execution-contract.md:19:   - If stale, refresh once; if still stale, reject with a stale-preview error.
docs/design/imp-semantic-write-execution-contract.md:21:   - Ask the hosted semantic adapter for an edit preview only.
docs/design/imp-semantic-write-execution-contract.md:22:   - Convert backend edits into a normalized preview envelope with affected paths, ranges, summary, risk flags, and size estimates.
docs/design/imp-semantic-write-execution-contract.md:26:   - Reject previews that are too large, ambiguous, partial, or include unsupported operations.
docs/design/imp-semantic-write-execution-contract.md:29:   - If approval is required, show the operator-visible preview and require explicit acknowledgement tied to the preview fingerprint.
docs/design/imp-semantic-write-execution-contract.md:32:   - Label it with action kind, target, preview fingerprint, timestamp, and verify command.
docs/design/imp-semantic-write-execution-contract.md:34:   - Apply the normalized hosted edit set through imp’s workspace edit path.
docs/design/imp-semantic-write-execution-contract.md:35:   - Re-check that target files have not drifted since preview.
docs/design/imp-semantic-write-execution-contract.md:43:    - Emit a `SemanticWriteResult` receipt with preview, approval, checkpoint, apply, refresh, and verify outcomes.
docs/design/imp-semantic-write-execution-contract.md:48:- A preview is bound to a workspace root, file content hashes, backend generation, action kind, and target location.
docs/design/imp-semantic-write-execution-contract.md:49:- Recompute preview when any affected file changes, the backend reports a newer semantic generation, or the operator changes action parameters.
docs/design/imp-semantic-write-execution-contract.md:50:- Reject instead of recomputing when the backend cannot prove freshness, preview generation repeatedly changes affected paths, or the target symbol/range no longer resolves.
docs/design/imp-semantic-write-execution-contract.md:51:- Approval is invalidated by any recomputed preview. The operator must acknowledge the new preview fingerprint.
docs/design/imp-semantic-write-execution-contract.md:56:- Capture all files that may be edited by the normalized preview, plus minimal metadata needed to explain/restore the action.
docs/design/imp-semantic-write-execution-contract.md:57:- Label format should include: `semantic-write:<action-kind>:<target>:<preview-fingerprint>`.
docs/design/imp-semantic-write-execution-contract.md:63:- `ApprovalPosture::AutoAllow` may proceed only for allowlisted low-risk actions with bounded preview size and fresh backend state.
docs/design/imp-semantic-write-execution-contract.md:64:- `ApprovalPosture::PreviewRequired` requires rendering the preview but may continue after explicit model/operator acknowledgement if policy allows.
docs/design/imp-semantic-write-execution-contract.md:65:- `ApprovalPosture::OperatorRequired` requires a human-visible acknowledgement of the exact preview fingerprint.
docs/design/imp-semantic-write-execution-contract.md:67:- Any preview drift, stale state, policy change, or target change clears approval.
docs/design/imp-semantic-write-execution-contract.md:72:- Backends must not run arbitrary write commands, shell commands, package-manager commands, or formatter commands as the apply mechanism.
docs/design/imp-semantic-write-execution-contract.md:73:- All edits pass through the same workspace boundary checks as other hosted edits.
docs/design/imp-semantic-write-execution-contract.md:79:- **Apply drift**: abort before writing, recompute preview if safe, otherwise require new approval.
docs/design/imp-semantic-write-execution-contract.md:83:- **Approval rejection**: no checkpoint or apply; record rejected preview as non-durable runtime output unless explicitly promoted.
docs/design/imp-semantic-write-execution-contract.md:92:- preview fingerprint and affected paths;
docs/design/imp-semantic-write-execution-contract.md:107:2. Capability lookup allows rename for the language/backend but requires operator preview.
docs/design/imp-semantic-write-execution-contract.md:109:4. Operator approves preview fingerprint.
docs/design/imp-semantic-write-execution-contract.md:116:2. Policy allows auto-apply only if preview affects that file and contains import-order edits only.
docs/design/imp-semantic-write-execution-contract.md:125:3. If preview includes unrelated edits, reject as policy drift.
docs/design/imp-work-mana-feature-parity.md:1:# imp-work vs mana feature parity matrix
docs/design/imp-work-mana-feature-parity.md:3:This document tracks what `imp-work` must match or surpass before imp can remove its mana runtime dependency. It is intentionally broad: mana is not just a file format, it is a bundle of graph, scheduling, verification, run, UX, and durable-history conventions that imp has relied on.
docs/design/imp-work-mana-feature-parity.md:17:| `.mana` durable project graph | `.imp/work` store primitives (`WorkStore`, tasks, memory, decisions, prototypes, runs, leases) | partial | Finish migration importer and write-mode conversion; define canonical `.imp/work` layout/versioning. | Gas City centers durable state in Beads; imp-work should make `.imp/work` the universal imp work substrate. |
docs/design/imp-work-mana-feature-parity.md:18:| Units with id/title/description/design/acceptance | `Task`, `Epic`, `WorkItem`, `ManaShadowUnit` importer mapping | partial | Preserve all mana fields in importer; add explicit design/description fields or durable memory mapping policy. | Keep work primitive central; don't couple behavior to orchestration roles. |
docs/design/imp-work-mana-feature-parity.md:21:| `mana next` ready work selection | scheduler primitives and workflow readiness helpers | partial | Implement critical-path/priority ready queue for imp-work and explain why tasks are blocked. | Mana `ReadyQueue` + Gas City ready bead query both separate observation from dispatch. |
docs/design/imp-work-mana-feature-parity.md:22:| Dependency-ordered run waves | early `imp-work::Scheduler`; mana has `RunPlan`/`RunWave` | missing/partial | Add wave planning over imp-work tasks, respecting deps, priority, path conflicts, and ready status. | Gas City dependency-aware bounded parallel lifecycle: plan serially, execute concurrently, commit serially. |
docs/design/imp-work-mana-feature-parity.md:23:| `mana run --jobs N` multi-agent dispatch | leases/runs exist as primitives, but no full multi-agent runner | missing | Implement `imp work run --jobs N` or tool action: ready waves, bounded concurrency, worker assignment, progress events, result aggregation. | Gas City agent pools: min/max, demand query, runtime provider abstraction, worker boundary, drain behavior. |
docs/design/imp-work-mana-feature-parity.md:27:| File-lock/path conflict dispatch | mana-pool has file locking/path metadata | missing | Use task paths to avoid concurrent conflicting jobs; report conflict blockers. | Plan serially and commit deterministically; worker completion order must not affect state. |
docs/design/imp-work-mana-feature-parity.md:28:| Retry context | mana run derives attempts/failure notes; imp-work runs record outcomes | partial | Add retry policy/history to scheduler and importer; preserve prior mana attempts. | Gas City leaves retry to next reconciler tick with explicit terminal results. |
docs/design/imp-work-mana-feature-parity.md:36:| Artifacts/evidence refs | `SourceRef`, run changed paths, prototype evidence | partial | Add explicit artifact refs and durable evidence summary records; migrate mana artifacts. | Gas City event bus and Beads store keep observable history separate from runtime process state. |
docs/design/imp-work-mana-feature-parity.md:39:| Memory/context search | imp has session/memory work; imp-work memory index exists | partial | Add work-memory search/filter and migration from mana notes/decisions/facts. | Gas City event/query docs separate stable machine output from human renderings. |
docs/design/imp-work-mana-feature-parity.md:40:| Archive/history | mana archives closed units and stores attempts | missing/partial | Define imp-work archive/history policy, compaction, and migration of closed `.mana` units. | Gas City append-only event bus suggests history should be observable, not destructive moves only. |
docs/design/imp-work-mana-feature-parity.md:43:| Review gates | mana has review/verify workflows | missing/partial | Add review status/action and evidence requirements if imp still needs review lanes. | Gas City review quorum formula shows review can be a formula/molecule, not hardcoded runtime logic. |
docs/design/imp-work-mana-feature-parity.md:45:| Worktree isolation | mana-pool supports worktrees | missing | Add optional per-worker git worktree/sandbox strategy before true parallel editing. | Gas City runtime providers keep side effects in Layer 0; imp-work should keep execution substrate separate. |
docs/design/imp-work-mana-feature-parity.md:47:| MCP/API embedding | mana has MCP/API surfaces | omit/partial | Decide if imp-work needs an external API or only imp-native tools. Do not rebuild mana platform accidentally. | Gas City has HTTP/SSE for supervisor; imp may only need local tool/CLI first. |
docs/design/imp-work-mana-feature-parity.md:48:| Durable event stream | mana logs/attempts; gascity has event bus | missing | Add append-only work event log for task/run/check/lease/prototype events. | Strong recommendation from gascity: event bus as universal observation substrate. |
docs/design/imp-work-mana-feature-parity.md:49:| Import/export/migration | `mana_shadow` dry-run/write importer started | partial | Finish robust parser/API path, parity reports, write mode, count checks, archive import, rollback/backups. | Migration should be one-way; no long-lived two-way sync unless forced. |
docs/design/imp-work-mana-feature-parity.md:54:Critical gaps before mana removal:
docs/design/imp-work-mana-feature-parity.md:56:1. **Complete migration importer** — dry-run plus write mode from `.mana` to `.imp/work`, including archive/history and parity report.
docs/design/imp-work-mana-feature-parity.md:61:6. **Archive/history model** — preserve closed/failed/attempt history from mana and define imp-work retention/compaction.
docs/design/imp-work-mana-feature-parity.md:62:7. **Machine output contract** — stable structured run/work events equivalent or better than mana's JSON/run stream.
docs/design/imp-work-mana-feature-parity.md:63:8. **Removal ledger** — explicit checklist proving mana tool, mana_worker, and mana-core dependency can be removed.
docs/design/imp-work-mana-feature-parity.md:67:1. Finish `359.1`: `.mana` -> `.imp/work` importer and parity/loss report.
docs/design/imp-work-mana-feature-parity.md:75:9. Run migration on a real `.mana` project in dry-run and write mode; compare counts and spot-check behavior.
docs/design/imp-work-mana-feature-parity.md:76:10. Switch new local imp work to `.imp/work`; keep `.mana` import-only.
docs/design/imp-work-mana-feature-parity.md:77:11. Remove mana runtime paths after removal criteria pass.
docs/design/imp-work-mana-feature-parity.md:92:1. **Work is the primitive, not orchestration.** Gas City says orchestration is a thin layer over the work substrate. imp-work should avoid rebuilding mana as a separate platform; task/check/memory/event data should be the center.
docs/design/imp-work-mana-feature-parity.md:94:3. **Plan serially, execute concurrently, commit serially.** This is the strongest pattern for imp-work multi-agent runs. Build dependency waves deterministically, run workers in bounded parallelism, then apply state transitions in planned order.
docs/design/imp-work-mana-feature-parity.md:99:8. **Conformance tests.** Gas City uses conformance matrices for providers/events/workers. imp-work should add conformance-style tests for store, scheduler, worker runner, and event log before mana removal.
docs/design/imp-work-mana-feature-parity.md:104:Do not remove mana from imp until all are true:
docs/design/imp-work-mana-feature-parity.md:106:- `.mana` -> `.imp/work` migration imports active and archived units with acceptable parity report.
docs/design/imp-work-mana-feature-parity.md:108:- Native work tool exposes the workflows the agent loop needs without calling mana.
docs/design/imp-work-mana-feature-parity.md:110:- Existing imp workflows that currently call `mana` have imp-work equivalents or explicit deprecation/migration paths.
docs/design/imp-work-mana-feature-parity.md:112:- `mana` tool usage in imp is narrowed to import-only/legacy mode.
docs/design/imp-work-mana-feature-parity.md:113:- Only then remove `crates/imp-core/src/tools/mana.rs`, `mana_worker`, mana-core dependency, and mana-specific runtime assumptions.
docs/design/imp-host-sync-mirror-daemon.md:6:Purpose: define the hosted/self-hosted architecture for syncing imp work across devices and teams while keeping agent computation on trusted machines by default.
docs/design/imp-host-sync-mirror-daemon.md:15:- **imp host** — optional hosted or self-hosted sync/control service.
docs/design/imp-host-sync-mirror-daemon.md:16:- **imp mirror** — optional git object/ref mirror managed by imp host.
docs/design/imp-host-sync-mirror-daemon.md:24:imp should remain useful as a fully local coding agent. The hosted product should not start as a hosted autonomous engineer. The first hosted product should be **imp host**: a durable coordination service for imp work, run events, evidence, approvals, devices, and optional code mirrors.
docs/design/imp-host-sync-mirror-daemon.md:30:This gives solo developers and small teams cross-device continuity without asking them to hand arbitrary code execution to a SaaS worker.
docs/design/imp-host-sync-mirror-daemon.md:34:- Sync imp work across devices and small teams.
docs/design/imp-host-sync-mirror-daemon.md:35:- Keep local imp useful without hosted services.
docs/design/imp-host-sync-mirror-daemon.md:40:- Make hosted execution optional and later, not foundational.
docs/design/imp-host-sync-mirror-daemon.md:47:- Do not require private source code mirroring for basic hosted work sync.
docs/design/imp-host-sync-mirror-daemon.md:50:- Do not make hosted SaaS execution the default trust model.
docs/design/imp-host-sync-mirror-daemon.md:78:- team/project permissions
docs/design/imp-host-sync-mirror-daemon.md:84:- runs on a user's Mac/Linux/Windows machine or self-hosted server
docs/design/imp-host-sync-mirror-daemon.md:157:Future optional worker managed by the service.
docs/design/imp-host-sync-mirror-daemon.md:159:Should require stronger isolation, policy, and approval boundaries than imp daemon. Hosted execution is not required for the first hosted product.
docs/design/imp-host-sync-mirror-daemon.md:211:Local imp can continue using the current imp-work store. A future local sync cache may use SQLite or an append-only local event journal, but the hosted service should not be constrained by the local storage format.
docs/design/imp-host-sync-mirror-daemon.md:315:executor_kind: daemon | hosted | local_upload | human
docs/design/imp-host-sync-mirror-daemon.md:773:For future hosted execution:
docs/design/imp-host-sync-mirror-daemon.md:813:Paid/self-hosted:
docs/design/imp-host-sync-mirror-daemon.md:821:- team projects
docs/design/imp-host-sync-mirror-daemon.md:824:- later hosted/self-hosted runners
docs/design/imp-host-sync-mirror-daemon.md:832:- hosted domain model
docs/design/imp-host-sync-mirror-daemon.md:839:### Phase 1: hosted imp work sync
docs/design/imp-host-sync-mirror-daemon.md:907:- local daemon result survives as host-managed ref
docs/design/imp-host-sync-mirror-daemon.md:922:### Phase 7: hosted/self-hosted execution expansion
docs/design/imp-host-sync-mirror-daemon.md:924:- self-hosted daemon packages
docs/design/imp-host-sync-mirror-daemon.md:925:- optional managed hosted runners
docs/design/imp-host-sync-mirror-daemon.md:927:- billing/retention/team controls
docs/design/imp-host-sync-mirror-daemon.md:935:- Should local imp continue to support file-backed imp work indefinitely? Recommendation: yes; hosted sync is optional.
docs/design/imp-host-sync-mirror-daemon.md:936:- Should hosted work sync require auth to a central service, or allow arbitrary host URLs first? Recommendation: design protocol for arbitrary host URLs even if first deployment is official.
docs/design/imp-work-global-store.md:5:imp-work should use a single user-global work store by default, with `project_root` recorded as first-class metadata on stored work records. This avoids mana's cwd-scoped `.mana` fragmentation where work created from `~` is invisible when imp is later launched from `~/imp`, and vice versa.
docs/design/imp-work-global-store.md:28:- Imported `.mana` records should preserve source refs and record the project root of the source graph.
docs/design/imp-work-global-store.md:34:Rejected as the default because it recreates the `.mana` cwd fragmentation problem. It can be retained as an import/export or project-local backup format.
docs/design/imp-work-global-store.md:46:1. Import old `.mana` graphs into the global store with `project_root` set to the source project.
docs/dependency-audit.md:17:- `serde_yml 0.0.12` (`RUSTSEC-2025-0068`) and `libyml 0.0.5` (`RUSTSEC-2025-0067`) come from `mana-core 0.3.2`.
docs/dependency-audit.md:18:  - No newer `mana-core` version was published at the time of review.
docs/dependency-audit.md:19:  - Mitigation requires an upstream `mana-core` release or reducing/removing mana-core integration.
docs/design/oss-launch-checklist.md:30:  - Teams experimenting with durable agent workflows before adopting hosted platforms.
docs/design/oss-launch-checklist.md:32:  - Not a hosted enterprise platform.
docs/design/oss-launch-checklist.md:56:- [ ] Add a "What is experimental" section.
docs/design/oss-launch-checklist.md:74:- [ ] Make sure old mana language is removed or clearly marked legacy.
docs/design/oss-launch-checklist.md:144:- [ ] `docs/work/overview.md`: native imp-work overview, no mana terminology.
docs/design/oss-launch-checklist.md:154:- [ ] `docs/extensibility/agents.md`: `.imp/agents` if available or planned.
docs/design/oss-launch-checklist.md:168:- [ ] Docs avoid stale future-tense promises unless clearly marked planned.
docs/design/oss-launch-checklist.md:169:- [ ] Docs use native imp-work vocabulary, not mana, except in migration/legacy notes.
docs/design/oss-launch-checklist.md:236:- [ ] Remove or clearly deprecate mana-first flows from launch docs.
docs/design/oss-launch-checklist.md:287:- [ ] Avoid claiming TypeScript extensions are shipped unless they are actually implemented.
docs/design/oss-launch-checklist.md:289:- [ ] If MCP is not implemented by launch, mark it as roadmap, not current capability.
docs/design/oss-launch-checklist.md:314:- [ ] Make launch docs distinguish stable, experimental, and legacy.
docs/design/oss-launch-checklist.md:350:- [ ] Help wanted: MCP design/implementation if not started.
docs/design/oss-launch-checklist.md:351:- [ ] Help wanted: ACP adapter research.
docs/design/oss-launch-checklist.md:377:  - "imp is local-first and inspectable; hosted platforms are broader team products."
docs/design/oss-launch-checklist.md:382:  - MCP
docs/design/oss-launch-checklist.md:423:8. Add roadmap with `.imp/agents`, MCP, Work Control, `/review`, ACP, and Slack.
docs/rebuild/imp-session-storage-search-recovery-audit.md:3:This audit resolves mana unit `50.16.5.1`: why `session_search` can report no indexed sessions even when raw session transcripts exist, and where an operator should look for recovery on this machine.
docs/design/imp-work-implementation-plan.md:3:`imp-work` is the native replacement for mana inside imp. The center is prepared work: durable memory, tasks, prototypes, context packs, runs, leases, and structured outcomes that let imp coordinate many low-memory subagents without losing conversational context.
docs/design/imp-work-implementation-plan.md:34:Defer broad product surfaces until the foundation is proven: full TUI replacement, embeddings/semantic search, massive live concurrency, container sandboxing, and committed mana import code.
docs/design/imp-work-implementation-plan.md:47:Avoid user-facing `mana`, `unit`, or generic `graph` terminology.
docs/design/imp-work-implementation-plan.md:51:imp-work must preserve the best mana behavior: while chatting, the agent can capture durable ideas that would otherwise be forgotten. The improvement is retrieval: the user should not need to remember where a note was stored.
docs/design/imp-work-implementation-plan.md:107:- supports shell, Python, Rust, JavaScript, TypeScript, Go, Elixir, Ruby, Perl, Lua, Zig, Odin, and Swift when local runtimes exist
docs/design/imp-work-implementation-plan.md:108:- TypeScript prefers `node --experimental-strip-types`, then bun, then deno
docs/design/imp-work-implementation-plan.md:204:.imp/work/prototypes.md  # planned/running/observed/promoted/discarded prototypes and observations
docs/design/imp-work-implementation-plan.md:221:`imp-core` now exposes a native `work` tool backed by `imp_work::WorkStore`. This is the first user/agent-facing replacement surface for mana task and memory workflows.
docs/design/imp-work-implementation-plan.md:263:1. Route selected agent memory/task behaviors from mana to the native `work` tool / `imp_work::WorkStore`.
docs/design/imp-work-implementation-plan.md:268:6. Keep mana migration as an off-repo/local script only.
docs/design/imp-work-implementation-plan.md:272:Mana is a reference and local migration source, not a runtime dependency. Do not commit `import/mana.rs` or a mana import module into `crates/imp-work`.
docs/rebuild/mana-embedding-surface-audit.md:1:# mana-core Embedding Surface Audit
docs/rebuild/mana-embedding-surface-audit.md:3:This audit compares the current `mana-core` embedding surface against the target semantic, lease-based, library-first contract recorded in root mana unit `51.6`.
docs/rebuild/mana-embedding-surface-audit.md:7:- `../mana/crates/mana-core/src/api/mod.rs`
docs/rebuild/mana-embedding-surface-audit.md:8:- `../mana/crates/mana-core/src/ops/mod.rs`
docs/rebuild/mana-embedding-surface-audit.md:9:- `../mana/crates/mana-core/src/ops/run.rs`
docs/rebuild/mana-embedding-surface-audit.md:10:- `../mana/crates/mana-cli/src/commands/run/mod.rs`
docs/rebuild/mana-embedding-surface-audit.md:11:- `../mana/.mana/51.6-define-the-next-mana-core-embedding-slice-add-stab.md`
docs/rebuild/mana-embedding-surface-audit.md:15:- `docs/rebuild/mana-lease-model.md`
docs/rebuild/mana-embedding-surface-audit.md:16:- `docs/rebuild/mana-imp-ownership-boundary.md`
docs/rebuild/mana-embedding-surface-audit.md:18:The target contract details were available from `../mana/.mana/51.6...` and are used below instead of inventing a new contract.
docs/rebuild/mana-embedding-surface-audit.md:48:- dynamic graph changes are proposals validated by mana;
docs/rebuild/mana-embedding-surface-audit.md:56:`mana-core/src/api/mod.rs` already presents itself as a programmatic API for embedding mana in another application. It emphasizes:
docs/rebuild/mana-embedding-surface-audit.md:70:### Scheduler computation is already in mana-core
docs/rebuild/mana-embedding-surface-audit.md:82:This is directionally correct: clients should ask mana for readiness rather than duplicating graph semantics.
docs/rebuild/mana-embedding-surface-audit.md:86:`mana-cli/src/commands/run/mod.rs` documents direct mode spawning `imp run <id>` when no template config is present. That matches the ownership direction where imp owns live execution and mana owns durable work state, but it is still a CLI spawn behavior rather than a stable library attach contract.
docs/rebuild/mana-embedding-surface-audit.md:110:No inspected `mana-core` API exposes a run event stream with cursors.
docs/rebuild/mana-embedding-surface-audit.md:117:Current CLI JSON stream behavior is not the same as a durable mana-owned event stream.
docs/rebuild/mana-embedding-surface-audit.md:129:`ReadyUnit` includes scheduling metadata and model override, but there is no executor capabilities handshake that lets mana decide whether an executor may attach to a run/node.
docs/rebuild/mana-embedding-surface-audit.md:141:### `mana-cli run` is CLI-shaped
docs/rebuild/mana-embedding-surface-audit.md:143:`mana-cli/src/commands/run/mod.rs` owns terminal/CLI behavior:
docs/rebuild/mana-embedding-surface-audit.md:164:Existing close/update/status operations are useful maintenance primitives, but the target explicitly rejects raw canonical state setters as the executor contract. Executors should resolve leases and propose evidence/graph deltas; mana should validate and own the durable state transition.
docs/rebuild/mana-embedding-surface-audit.md:170:1. A top-level run service/module in `mana-core` that can create a run from existing `RunTarget`/`RunPlan` output.
docs/rebuild/mana-embedding-surface-audit.md:187:Instead, define one stable library-facing spawn/attach boundary in `mana-core`:
docs/rebuild/mana-embedding-surface-audit.md:189:- `mana_core::run_service` or `mana_core::api::runs`
docs/rebuild/mana-embedding-surface-audit.md:195:The CLI and RPC layers can later adapt to the same service, but the semantic contract should be implemented in `mana-core`, not in `mana-cli` or `mana-pool`.
docs/rebuild/mana-embedding-surface-audit.md:201:1. Add mana-core types only:
docs/rebuild/mana-embedding-surface-audit.md:213:   - `create_run(mana_dir, spec)` using current `compute_run_plan` semantics;
docs/rebuild/mana-embedding-surface-audit.md:214:   - `get_snapshot(mana_dir, run_id)`;
docs/rebuild/mana-embedding-surface-audit.md:215:   - `attach_executor(mana_dir, run_id, capabilities)` for a single ready node;
docs/rebuild/mana-embedding-surface-audit.md:216:   - `heartbeat(mana_dir, lease_id, progress)`;
docs/rebuild/mana-embedding-surface-audit.md:217:   - `resolve_lease(mana_dir, lease_id, outcome)`.
docs/rebuild/mana-embedding-surface-audit.md:218:3. Persist enough run/lease state to prove lifecycle, but do not replace `mana run` dispatch yet.
docs/rebuild/mana-embedding-surface-audit.md:219:4. Add unit tests using a temporary `.mana` fixture.
docs/rebuild/mana-embedding-surface-audit.md:224:cargo test -p mana-core run_service -- --nocapture
docs/rebuild/mana-embedding-surface-audit.md:225:cargo check -p mana-cli
docs/rebuild/mana-embedding-surface-audit.md:238:The first stable boundary belongs in `mana-core`.
docs/rebuild/mana-embedding-surface-audit.md:243:- `mana-cli` is presentation/orchestration compatibility, not canonical state semantics.
docs/rebuild/mana-embedding-surface-audit.md:244:- `mana-pool` may execute pools/workers but should not own durable graph/run legality.
docs/rebuild/mana-embedding-surface-audit.md:245:- `imp` owns live agent execution, not mana run/lease truth.
docs/rebuild/mana-embedding-surface-audit.md:247:After the mana-core service exists:
docs/rebuild/mana-embedding-surface-audit.md:249:- `mana-cli run` can become a compatibility adapter over it;
docs/rebuild/mana-embedding-surface-audit.md:250:- `imp run {mana_id}` can attach through it;
docs/rebuild/mana-embedding-surface-audit.md:257:- Do not make `mana-cli run` the stable embedding contract.
docs/rebuild/mana-embedding-surface-audit.md:263:The current embedding surface is solid for unit/query/mutation operations and readiness planning, but it lacks the semantic run/lease/event/evidence layer required by the target contract. The next coherent slice is a small `mana-core` run/lease service that wraps current planning logic and proves create/snapshot/attach/heartbeat/resolve locally before changing CLI or imp execution behavior.
docs/design/dirac-inspired-code-tools.md:169:- `scan` skeleton/symbol output improvements with Rust/TypeScript tests.
docs/rebuild/imp-output-mode-contract.md:7:- planning notes in `.mana/50.16.1` document duplicated headless/RPC JSON encoders and the target split;
docs/rebuild/imp-output-mode-contract.md:8:- `.mana/50.17` captures the follow-on output-contract requirement;
docs/rebuild/imp-output-mode-contract.md:13:The referenced historical docs (`imp-cli-affordance-sequence`, `imp-shared-runtime-startup-map`, `imp-command-grammar`, and shell transcript UX) are not present in this worktree, so this contract uses their durable mana summaries plus currently restored rebuild docs.
docs/design/imp-work-mana-migration-plan.md:1:# imp-work mana migration plan
docs/design/imp-work-mana-migration-plan.md:3:`imp-work` is now a native foundation for agent-local work, but imp is not ready to remove mana yet. Mana still owns durable project graphs, existing `.mana` data, verify/close/fail semantics, scheduling, and much of the operator workflow. The migration must be adapter-backed and reversible.
docs/design/imp-work-mana-migration-plan.md:7:Move imp’s default task orchestration from mana-backed workflows to native `imp-work` while preserving existing mana projects until parity is proven.
docs/design/imp-work-mana-migration-plan.md:11:- No immediate deletion of the `mana` tool, `mana-core` dependency, or `.mana` compatibility.
docs/design/imp-work-mana-migration-plan.md:12:- No flag-day conversion of existing `.mana` projects.
docs/design/imp-work-mana-migration-plan.md:13:- No direct destructive mutation from imp-work into mana during the first migration slices.
docs/design/imp-work-mana-migration-plan.md:14:- No claim that imp-work replaces mana until parity tests pass on real project units.
docs/design/imp-work-mana-migration-plan.md:18:Define a mana-to-imp-work adapter that maps:
docs/design/imp-work-mana-migration-plan.md:29:The adapter output should be an imp-work work item plus context pack, not a mana mutation.
docs/design/imp-work-mana-migration-plan.md:33:Add a shadow import path for selected mana units:
docs/design/imp-work-mana-migration-plan.md:35:1. read a mana unit and immediate graph context;
docs/design/imp-work-mana-migration-plan.md:38:4. report parity gaps without changing `.mana` state.
docs/design/imp-work-mana-migration-plan.md:44:Add an outcome bridge from imp-work back to mana-compatible proposals:
docs/design/imp-work-mana-migration-plan.md:51:The bridge initially emits proposals only. A human/operator or existing mana tool path applies them.
docs/design/imp-work-mana-migration-plan.md:55:For new imp-local tasks, prefer imp-work creation and scheduling. Keep mana commands available for existing `.mana` project work.
docs/design/imp-work-mana-migration-plan.md:59:- If a user explicitly names a mana unit or `.mana` graph, use compatibility/shadow bridge.
docs/design/imp-work-mana-migration-plan.md:60:- If a user asks for new local agent work without a mana unit, use imp-work native store.
docs/design/imp-work-mana-migration-plan.md:64:Before deprecating mana-backed paths in imp, prove parity for:
docs/design/imp-work-mana-migration-plan.md:80:1. mark mana-native imp tool usage as compatibility mode;
docs/design/imp-work-mana-migration-plan.md:81:2. provide migration/export guidance for `.mana` projects;
docs/design/imp-work-mana-migration-plan.md:83:4. remove mana runtime dependency only after existing project graphs have a supported bridge.
docs/design/imp-work-mana-migration-plan.md:90:- load a mana unit fixture into an imp-work item;
docs/design/imp-work-mana-migration-plan.md:92:- do not write to mana;
docs/design/imp-work-mana-migration-plan.md:97:We can start preparing to remove mana from imp by building the adapter and shadow import. We should not remove mana from imp yet.
docs/rebuild/imp-attach-path-cutover.md:3:This is an audit-plus-cutover sequence from today’s `imp run <unit-id>` and compatibility `mana run` coexistence toward a lease-based attach model where `imp run {mana_id}` attaches a live imp runtime/session to a mana-owned run.
docs/rebuild/imp-attach-path-cutover.md:9:- `crates/imp-core/src/mana_worker.rs` declares itself the canonical single-unit mana worker runtime. It loads mana units through `mana_core::api`, assembles task context, and reports structured worker outcomes.
docs/rebuild/imp-attach-path-cutover.md:10:- `../mana/crates/mana-core/src/ops/run.rs` owns scheduling legality primitives today: `ReadyQueue`, `ReadyUnit`, `RunPlan`, `RunWave`, blocked units, warnings, retry context, dependency satisfaction, and target matching.
docs/rebuild/imp-attach-path-cutover.md:11:- `../mana/crates/mana-cli/src/commands/run/mod.rs` still presents `mana run` as dispatch/spawn behavior. It supports template-mode compatibility and direct mode that spawns `imp run <id>`.
docs/rebuild/imp-attach-path-cutover.md:12:- `../mana/crates/mana-cli/src/commands/run/plan.rs` adapts `mana_core::ops::run` plans into CLI dispatch units and wave planning.
docs/rebuild/imp-attach-path-cutover.md:13:- The originally referenced rebuild docs were not present in this worktree, so this plan is grounded in the inspected code and the mana unit contract.
docs/rebuild/imp-attach-path-cutover.md:17:- `mana` owns durable/shared/coordinated execution truth: run records, node legality, leases, heartbeats, checkpoints, artifacts, verification records, and final resolution.
docs/rebuild/imp-attach-path-cutover.md:19:- `imp run {mana_id}` is the preferred live boundary. It attaches to a mana-owned run lease rather than independently deciding run legality.
docs/rebuild/imp-attach-path-cutover.md:20:- `mana run` becomes compatibility orchestration that creates/selects mana-owned runs and delegates live work to imp, then narrows or disappears once callers migrate.
docs/rebuild/imp-attach-path-cutover.md:27:- **Run**: mana-owned durable execution container for one target set, policy, scheduling snapshot, and aggregate outcome.
docs/rebuild/imp-attach-path-cutover.md:28:- **Run node**: mana-owned durable execution item corresponding to a unit attempt within a run.
docs/rebuild/imp-attach-path-cutover.md:29:- **Lease**: mana-owned exclusive right for a worker/runtime to execute or verify a run node for a bounded heartbeat interval.
docs/rebuild/imp-attach-path-cutover.md:30:- **Heartbeat**: mana-owned liveness update from imp for a lease.
docs/rebuild/imp-attach-path-cutover.md:33:- **Checkpoint**: mana-indexed restore/recovery anchor created or referenced by imp at important execution boundaries.
docs/rebuild/imp-attach-path-cutover.md:34:- **Resolution**: mana-owned final state transition for a node/run: closed, failed, abandoned, cancelled, or awaiting verify.
docs/rebuild/imp-attach-path-cutover.md:38:Owner: imp with mana compatibility.
docs/rebuild/imp-attach-path-cutover.md:40:Current canonical live path remains `imp run <unit-id>` through `imp-core/src/mana_worker.rs`.
docs/rebuild/imp-attach-path-cutover.md:44:- `mana run` direct mode spawning `imp run <id>`.
docs/rebuild/imp-attach-path-cutover.md:46:- Current `ReadyQueue`/`RunPlan` scheduling helpers in mana-core.
docs/rebuild/imp-attach-path-cutover.md:50:- scheduler legality decisions duplicated between imp and mana;
docs/rebuild/imp-attach-path-cutover.md:52:- final outcome recording outside mana-owned run/node semantics.
docs/rebuild/imp-attach-path-cutover.md:59:## Phase 1 — Canonical Run/Node Schema in mana
docs/rebuild/imp-attach-path-cutover.md:61:Owner: mana.
docs/rebuild/imp-attach-path-cutover.md:67:- `../mana/crates/mana-core/src/ops/run.rs`
docs/rebuild/imp-attach-path-cutover.md:68:- new mana-core run-record module or storage helpers
docs/rebuild/imp-attach-path-cutover.md:69:- mana CLI display/status surfaces
docs/rebuild/imp-attach-path-cutover.md:81:- `mana run --dry-run` or a new internal API can create a run plan record in dry-run/shadow mode and print it without dispatching.
docs/rebuild/imp-attach-path-cutover.md:94:Owner: shared mana + imp contract work.
docs/rebuild/imp-attach-path-cutover.md:96:Keep current `imp run <unit-id>` execution, but have it record into mana-owned run/node semantics in parallel with existing unit attempt/close behavior.
docs/rebuild/imp-attach-path-cutover.md:100:- `crates/imp-core/src/mana_worker.rs`
docs/rebuild/imp-attach-path-cutover.md:101:- mana-core run APIs from Phase 1
docs/rebuild/imp-attach-path-cutover.md:102:- mana close/verify outcome adapters
docs/rebuild/imp-attach-path-cutover.md:107:- mana creates or resolves a compatibility run/node for the unit;
docs/rebuild/imp-attach-path-cutover.md:122:- result mapping from imp `WorkerResult` to mana run-node resolution.
docs/rebuild/imp-attach-path-cutover.md:126:Owner: mana.
docs/rebuild/imp-attach-path-cutover.md:128:Make mana compute authoritative legality/readiness and compare it against existing dispatch decisions without enforcing leases yet.
docs/rebuild/imp-attach-path-cutover.md:132:- `../mana/crates/mana-core/src/ops/run.rs`
docs/rebuild/imp-attach-path-cutover.md:133:- `../mana/crates/mana-cli/src/commands/run/plan.rs`
docs/rebuild/imp-attach-path-cutover.md:134:- imp native mana tool/run orchestration surfaces if they show run state
docs/rebuild/imp-attach-path-cutover.md:138:- current `mana run` and `imp run` flows ask mana for legality/readiness snapshots;
docs/rebuild/imp-attach-path-cutover.md:140:- unresolved decisions, dependency closure, scope warnings, artifact requirements, and retry policy are all evaluated by mana.
docs/rebuild/imp-attach-path-cutover.md:152:- scheduler legality cannot remain split across imp and mana.
docs/rebuild/imp-attach-path-cutover.md:156:Owner: shared mana + imp; imp owns live worker, mana owns lease state.
docs/rebuild/imp-attach-path-cutover.md:158:Change the preferred `imp run {mana_id}` path so imp attaches to a mana-owned node lease.
docs/rebuild/imp-attach-path-cutover.md:162:- `crates/imp-core/src/mana_worker.rs`
docs/rebuild/imp-attach-path-cutover.md:164:- mana-core lease APIs
docs/rebuild/imp-attach-path-cutover.md:165:- mana CLI status/run-state display
docs/rebuild/imp-attach-path-cutover.md:169:1. imp asks mana to create/select a run node for `{mana_id}`.
docs/rebuild/imp-attach-path-cutover.md:171:3. mana grants or rejects based on legality/readiness/current holder.
docs/rebuild/imp-attach-path-cutover.md:173:5. imp records checkpoints/artifacts through mana APIs.
docs/rebuild/imp-attach-path-cutover.md:174:6. imp resolves the node through mana with structured outcome.
docs/rebuild/imp-attach-path-cutover.md:175:7. mana updates unit state/attempt logs as a derived effect or compatibility projection.
docs/rebuild/imp-attach-path-cutover.md:179:- One single-unit `imp run <unit-id>` attach path for a local `.mana` unit: acquire lease, heartbeat once, execute current worker path, resolve lease with final worker status, and preserve existing verify/close behavior.
docs/rebuild/imp-attach-path-cutover.md:183:- If no lease API is available, current path can still run behind an experimental flag.
docs/rebuild/imp-attach-path-cutover.md:190:## Phase 5 — mana run Becomes Attach-Orchestrator Compatibility
docs/rebuild/imp-attach-path-cutover.md:192:Owner: mana CLI with imp worker contract.
docs/rebuild/imp-attach-path-cutover.md:194:Turn `mana run` into an orchestrator over mana-owned runs and imp attach workers rather than an independent spawn/dispatch owner.
docs/rebuild/imp-attach-path-cutover.md:198:- `../mana/crates/mana-cli/src/commands/run/mod.rs`
docs/rebuild/imp-attach-path-cutover.md:199:- `../mana/crates/mana-cli/src/commands/run/ready_queue.rs`
docs/rebuild/imp-attach-path-cutover.md:200:- `../mana/crates/mana-cli/src/commands/run/wave.rs`
docs/rebuild/imp-attach-path-cutover.md:201:- `../mana/crates/mana-cli/src/commands/run/plan.rs`
docs/rebuild/imp-attach-path-cutover.md:205:- `mana run` computes/creates run nodes;
docs/rebuild/imp-attach-path-cutover.md:208:- render run status from mana-owned state;
docs/rebuild/imp-attach-path-cutover.md:213:- Direct mode dispatches one wave through lease-based imp attach and reports outcomes entirely from mana run-node state.
docs/rebuild/imp-attach-path-cutover.md:221:- `mana run` must no longer invent lifecycle states not representable in mana run/node records.
docs/rebuild/imp-attach-path-cutover.md:225:Owner: imp for transcript/session, mana for recovery artifacts.
docs/rebuild/imp-attach-path-cutover.md:227:Clarify and enforce that imp transcripts are runtime/session artifacts, not mana’s canonical recovery substrate.
docs/rebuild/imp-attach-path-cutover.md:232:- mana stores structured checkpoints, verify outputs, artifacts, and final evidence summaries;
docs/rebuild/imp-attach-path-cutover.md:233:- recovery starts from mana run/node/checkpoint state and may ask imp to resume/reconstruct context, but not by replaying transcript as truth.
docs/rebuild/imp-attach-path-cutover.md:237:- A failed lease-based run records checkpoint/ref + verify failure artifact in mana while imp keeps transcript/session local.
docs/rebuild/imp-attach-path-cutover.md:239:## Phase 7 — Narrow or Deprecate mana run
docs/rebuild/imp-attach-path-cutover.md:241:Owner: mana CLI and docs.
docs/rebuild/imp-attach-path-cutover.md:243:Once attach orchestration is stable, decide the remaining role for `mana run`.
docs/rebuild/imp-attach-path-cutover.md:259:1. Add mana-core APIs for a single run node lease: create/select node, acquire, heartbeat, resolve.
docs/rebuild/imp-attach-path-cutover.md:260:2. Add an experimental imp path that calls those APIs around the existing `mana_worker` execution.
docs/rebuild/imp-attach-path-cutover.md:263:5. Assert that mana records the lease lifecycle and final resolution.
docs/rebuild/imp-attach-path-cutover.md:265:This proves the ownership boundary without changing scheduling, parallel dispatch, transcript storage, or `mana run` user behavior.
docs/rebuild/imp-attach-path-cutover.md:271:- No scheduler legality split between imp and mana after shadow validation hardens.
docs/rebuild/imp-attach-path-cutover.md:272:- No direct backend/runtime ownership in mana.
docs/rebuild/imp-attach-path-cutover.md:273:- No `mana run` semantic expansion beyond compatibility orchestration.
docs/design/imp-native-workflow-engine.md:16:- The `workflow` tool may eventually replace normal imp use of `mana`, `work`, and `prototype`.
docs/design/imp-native-workflow-engine.md:53:GitHub Agentic Workflows shows that repo behavior can be described above CI YAML. Goose and Gemini CLI reinforce local operator UX: checkpointing, resume, custom commands, status visibility, headless structured output, and extension/MCP integration. imp should compete by making workflows enforceable and inspectable, not just reusable prompts.
docs/design/imp-native-workflow-engine.md:71:The tool owns workflow artifacts directly, similar to how mana mutates its work graph. It may write:
docs/design/imp-native-workflow-engine.md:82:This tool could replace current agent-facing `mana`, `work`, and `prototype` responsibilities for imp-native work:
docs/design/imp-native-workflow-engine.md:86:- `mana` replacement for everyday imp execution: bounded execution, verification, review, and closeout live in imp. Mana remains optional indexing/aggregation if needed.
docs/design/imp-native-workflow-engine.md:267:## Relationship to mana/work/prototype
docs/design/imp-native-workflow-engine.md:269:This direction redefines the tool and mana/imp split:
docs/design/imp-native-workflow-engine.md:274:- the workflow tool can absorb everyday imp execution responsibilities currently routed through `mana`;
docs/design/imp-native-workflow-engine.md:275:- mana, if retained, owns optional indexing, aggregation, GUI/board views, and long-horizon project graph behavior;
docs/design/imp-native-workflow-engine.md:276:- mana should not be required for normal imp workflow execution.
docs/design/imp-native-workflow-engine.md:285:- `WorkflowRunController`: evolve into workflow engine or retire mana/imp-work-specific assumptions.
docs/design/imp-native-workflow-engine.md:287:- Existing `mana`, `work`, and `prototype` tool responsibilities: evaluate replacement by native workflow tool.
docs/design/imp-native-workflow-engine.md:320:12. Evaluate replacing existing `mana`, `work`, and `prototype` tools with workflow-tool actions.
docs/design/imp-native-workflow-engine.md:347:- How much of `mana`, `work`, and `prototype` should be retired once workflow-tool parity exists?
docs/rebuild/imp-rebuild-migration-sequence.md:19:2. Document `WorkerAssignment` as the live assignment snapshot from mana-owned durable state.
docs/rebuild/imp-rebuild-migration-sequence.md:20:3. Keep `mana_worker.rs` re-exports for compatibility shim stability.
docs/rebuild/imp-rebuild-migration-sequence.md:58:Goal: move toward mana-owned run/node/lease state while preserving `imp run <unit-id>`.
docs/rebuild/imp-rebuild-migration-sequence.md:61:1. Add mana run/node schema in shadow mode.
docs/rebuild/imp-rebuild-migration-sequence.md:65:5. Only then narrow `mana run` into attach orchestration.
docs/rebuild/imp-rebuild-migration-sequence.md:70:- `mana run` may continue spawning imp during transition.
docs/rebuild/imp-rebuild-migration-sequence.md:75:- mana dry-run/shadow validation tests
docs/rebuild/imp-rebuild-migration-sequence.md:80:- Disable dual-write or lease attach flag and fall back to current `mana_worker` execution.
docs/rebuild/imp-rebuild-migration-sequence.md:89:3. Promote summaries to mana only at checkpoint/verify/result boundaries.
docs/rebuild/imp-rebuild-migration-sequence.md:93:- Existing mana notes/logs remain accepted storage until a typed boundary has multiple consumers.
docs/rebuild/imp-rebuild-migration-sequence.md:157:- `mana run` compatibility does not disappear until attach orchestration is proven.
docs/rebuild/imp-rebuild-migration-sequence.md:171:- Changing `mana run` dispatch ownership.
docs/rebuild/imp-normalized-storage-contract.md:27:- `prompts/` — reserved/experimental prompt templates.
docs/rebuild/imp-normalized-storage-contract.md:28:- `tools/` — reserved/experimental shell-tool definitions; do not auto-enable without policy.
docs/rebuild/imp-normalized-storage-contract.md:43:- `prompts/` — reserved/experimental prompt templates.
docs/rebuild/imp-normalized-storage-contract.md:44:- `tools/` — reserved/experimental shell-tool definitions; policy-gated before activation.
docs/rebuild/imp-session-index-lifecycle-audit.md:3:This audit resolves mana unit `264.4`: determine whether imp indexes saved sessions during normal runtime lifecycle and define the smallest correct ownership seam for reliable `session_search`.
docs/rebuild/imp-session-index-lifecycle-audit.md:81:pub fn index_session_manager(session: &SessionManager) -> Result<IndexSessionOutcome>
docs/rebuild/imp-session-index-lifecycle-audit.md:87:- `index_session_manager` indexes the active session into `storage::global_session_index_path()` if the session has a persisted path.
docs/rebuild/imp-session-index-lifecycle-audit.md:103:   - index one persisted session manager;
docs/rebuild/imp-shared-ui-event-seam.md:72:   - Grounding: existing TUI runtime signals and tool-call rendering, mana stream handling, and CLI JSON stream serializers.
docs/rebuild/mana-imp-contract-boundary-map.md:1:# mana ↔ imp Contract Boundary Map
docs/rebuild/mana-imp-contract-boundary-map.md:12:- `crates/imp-core/src/mana_worker.rs` re-exports worker assignment/result types for existing call sites;
docs/rebuild/mana-imp-contract-boundary-map.md:13:- mana provides durable unit loading, verify/close operations, scheduling primitives, and future run/lease substrate;
docs/rebuild/mana-imp-contract-boundary-map.md:20:Current role: imp runtime input assembled from a mana unit.
docs/rebuild/mana-imp-contract-boundary-map.md:32:Boundary meaning: this is close to the future `TaskSpec` + `WorkerAssignment` split. Durable task facts originate in mana; imp should only receive a normalized assignment snapshot for live execution.
docs/rebuild/mana-imp-contract-boundary-map.md:36:- mana-owned task/run node record stores durable unit/run state;
docs/rebuild/mana-imp-contract-boundary-map.md:50:Boundary meaning: this is the current seed of a future `WorkerOutcome`. It should map into mana-owned run-node resolution, not become a second durable lifecycle universe.
docs/rebuild/mana-imp-contract-boundary-map.md:55:- mana records final node/run resolution and derives unit state changes.
docs/rebuild/mana-imp-contract-boundary-map.md:74:- mana owns verification records and artifact storage/indexing;
docs/rebuild/mana-imp-contract-boundary-map.md:107:Boundary meaning: this is a seed for durable evidence promotion across imp and mana. It overlaps with the later durable evidence summary direction but should stay small until multiple producers/consumers require typed exchange.
docs/rebuild/mana-imp-contract-boundary-map.md:111:- mana owns evidence bundle records;
docs/rebuild/mana-imp-contract-boundary-map.md:116:### mana owns durable substrate
docs/rebuild/mana-imp-contract-boundary-map.md:141:Because the current repository has `imp_core::contracts` but no neutral contracts crate, the least risky next step is to harden the imp-local contract module and explicitly map it to mana-owned future run/lease APIs. A crate move should wait until mana and imp both consume the same typed package in this worktree.
docs/rebuild/mana-imp-contract-boundary-map.md:147:1. Rename or document `WorkerAssignment` as the live assignment snapshot generated from mana-owned durable state.
docs/rebuild/mana-imp-contract-boundary-map.md:148:2. Add a typed conversion/mapping note or helper from `WorkerResult` + `VerifierResult` into a future mana run-node outcome shape.
docs/rebuild/mana-imp-contract-boundary-map.md:149:3. Keep all public call sites stable through existing `mana_worker.rs` re-exports.
docs/rebuild/mana-imp-contract-boundary-map.md:157:- `mana_worker::load_assignment` remains the adapter from mana unit state to imp runtime assignment;
docs/rebuild/mana-imp-contract-boundary-map.md:158:- `mana_worker` continues re-exporting `WorkerAssignment`, `WorkerAttempt`, `WorkerResult`, and `WorkerStatus`;
docs/rebuild/mana-imp-contract-boundary-map.md:159:- new mana run/lease APIs should accept a bounded outcome/evidence projection rather than imp internals;
docs/rebuild/mana-imp-contract-boundary-map.md:164:- Duplicating lifecycle status between `WorkerStatus` and future mana run-node state.
docs/rebuild/mana-imp-contract-boundary-map.md:165:- Persisting imp transcript details as mana canonical recovery state.
docs/rebuild/mana-imp-contract-boundary-map.md:168:- Letting `mana_worker.rs` continue accumulating durable substrate logic instead of delegating that to mana APIs.
docs/rebuild/imp-durable-storage-surface-audit.md:3:This audit inventories current durable imp-managed file surfaces and path-resolution behavior for mana unit `264.1`.
docs/rebuild/imp-workflow-feature-inventory.md:3:Status: decision inventory for mana task 365.9.
docs/rebuild/imp-workflow-feature-inventory.md:5:This inventory reconciles the current 0.3 direction with existing imp/mana/work/prototype surfaces. The working direction comes from `.imp/workflows`, especially:
docs/rebuild/imp-workflow-feature-inventory.md:14:The important direction change: imp-native workflows are the intended primary orchestration capability for imp 0.3. They may replace normal imp use of mana, work, and prototype once workflow parity exists. Older mana-first docs and tasks are historical context, not normative 0.3 product direction unless explicitly revived.
docs/rebuild/imp-workflow-feature-inventory.md:31:| mana integration | Optional mana command/tool/UI integration | Compatibility-only / optional adapter | 0.3 should not depend on mana for normal execution. mana may remain useful for old graphs or external experiments. | Keep behind `mana-ui` / `mana-tool`; default dependency tree must stay free of `mana-core` and `mana-cli`. |
docs/rebuild/imp-workflow-feature-inventory.md:32:| mana-first 365 child specs | Prior target architecture around mana harness | Defer/supersede for 0.3 | The active workflow artifacts contradict mana-first acceptance. Continuing those specs would create stale product direction. | Create a superseding workflow epic or rewrite 365 before doing more mana-harness spec work. |
docs/rebuild/imp-workflow-feature-inventory.md:33:| Runbooks | Executable plans previously discussed as mana feature | Fold into workflows | A workflow is already an executable plan with steps, checks, workers, context, and closeout. A separate runbook concept adds naming/abstraction debt. | Use “workflow template/profile” if reusable runbook-like behavior is needed. |
docs/rebuild/imp-workflow-feature-inventory.md:41:| Lua extensions | Current shipped extension runtime | Keep | Lua remains shipped extensibility. Workflow custom steps may eventually call extension tools, but workflow schema should remain Rust-validated YAML. | No removal. Avoid claiming TypeScript extensions are shipped. |
docs/rebuild/imp-workflow-feature-inventory.md:57:4. Keep mana command/tool/UI integration optional behind `mana-ui` / `mana-tool`.
docs/rebuild/imp-workflow-feature-inventory.md:60:7. Avoid implementing new mana-first harness docs under 365 until the epic is reconciled with the workflow direction.
docs/rebuild/imp-workflow-feature-inventory.md:76:Minimum parity before workflows can replace normal use of mana/work/prototype:
docs/rebuild/imp-workflow-feature-inventory.md:97:   - Describe workflows, bounded subagents, prototype steps, checks, and optional mana compatibility.
docs/rebuild/imp-workflow-feature-inventory.md:116:8. **Reconcile mana epic 365**
docs/rebuild/imp-workflow-feature-inventory.md:117:   - Either supersede old mana-first children or rewrite the epic into workflow-first 0.3 planning.
docs/rebuild/imp-workflow-feature-inventory.md:120:   - Explain how old prototype/work/mana users should map behavior to workflows.
docs/rebuild/imp-workflow-feature-inventory.md:126:- default standalone dependency tree still clean of `imp-work`, `mana-core`, and `mana-cli`;
docs/rebuild/imp-workflow-feature-inventory.md:130:- real bounded subagent execution or clear experimental marking;
