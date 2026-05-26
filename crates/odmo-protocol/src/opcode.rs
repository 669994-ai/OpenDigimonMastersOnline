pub const CHECKSUM_VALIDATION: i16 = 6716;

pub mod account {
    pub const KEEP_CONNECTION: i16 = -3;
    pub const CONNECTION: i16 = -1;
    pub const CONNECTED_EVENT: i16 = -1;
    pub const CONNECTION_RESPONSE: i16 = -2;
    pub const LOGIN_REQUEST: i16 = 3301;
    pub const LOAD_SERVER_LIST: i16 = 1701;
    pub const CONNECT_CHARACTER_SERVER: i16 = 1702;
}

pub mod character {
    pub const KEEP_CONNECTION: i16 = -3;
    pub const CONNECTION: i16 = -1;
    pub const CONNECTION_RESPONSE: i16 = -2;
    pub const CREATE_CHARACTER: i16 = 1303;
    pub const DELETE_CHARACTER: i16 = 1304;
    pub const CHECK_NAME_DUPLICITY: i16 = 1302;
    pub const GET_CHARACTER_POSITION: i16 = 1305;
    pub const REQUEST_CHARACTERS: i16 = 1706;
    pub const CONNECT_GAME_SERVER: i16 = 1703;
    pub const CHARACTER_LIST: i16 = 1301;
    pub const CHARACTER_CREATED: i16 = 1306;
    pub const CHARACTER_CREATION_FAILED: i16 = 1307;
    pub const CHARACTER_DELETED: i16 = 1304;
    pub const CONNECT_GAME_SERVER_INFO: i16 = 1308;
}

pub mod game {
    pub const KEEP_CONNECTION: i16 = -3;
    pub const CONNECTION: i16 = -1;
    pub const CONNECTION_RESPONSE: i16 = -2;
    pub const COMPLEMENTAR_INFORMATION: i16 = 1001;
    pub const TAMER_MOVIMENTATION: i16 = 1004;
    pub const MAP_ENTITY: i16 = 1006;
    pub const LOAD_UNLOAD_ENTITY: i16 = 1007;
    pub const CONSIGNED_SHOP_ENTITY: i16 = 1008;
    pub const LOAD_BUFFS: i16 = 1009;
    pub const NOTICE_MESSAGE: i16 = 1010;
    pub const CHAT_MESSAGE: i16 = 1012;
    pub const INITIAL_INFORMATION: i16 = 1706;
    pub const INITIAL_INFO_RESPONSE: i16 = 1003;
    pub const UPDATE_STATUS: i16 = 1043;
    pub const SERVER_EXPERIENCE: i16 = 1054;
    pub const SEALS: i16 = 1333;
    pub const AVAILABLE_RELATIONS: i16 = 2404;
    pub const FRIEND_CONNECT: i16 = 2408;
    pub const AVAILABLE_CHANNELS: i16 = 1713;
    pub const GUILD_INFORMATION: i16 = 2113;
    pub const GUILD_RANK: i16 = 2127;
    pub const GUILD_HISTORIC: i16 = 2128;
    pub const TIME_REWARD: i16 = 3106;
    pub const TAMER_ATTENDANCE: i16 = 3133;
    pub const CASH_SHOP_COINS: i16 = 3404;
    pub const MEMBERSHIP: i16 = 3414;
    pub const PICK_BITS: i16 = 3911;
    pub const PICK_ITEM_FAIL: i16 = 3913;
    pub const LOOT_ITEM: i16 = 3910;
    pub const UPDATE_MOVEMENT_SPEED: i16 = 9905;
    pub const LOAD_INVENTORY: i16 = 16009;
    pub const TAMER_XAI_RESOURCES: i16 = 16032;
    pub const XAI_INFO: i16 = 16033;
    pub const SYNC_CONDITION: i16 = 1070;
    pub const WARP_GATE: i16 = 1709;
    pub const LOCAL_MAP_SWAP: i16 = 1711;
    pub const CONSUME_ITEM: i16 = 3901;
    pub const MOVE_ITEM: i16 = 3904;
    pub const SPLIT_ITEM: i16 = 3907;
    pub const ITEM_REMOVE: i16 = 3909;
    pub const NPC_PURCHASE: i16 = 3915;
    pub const NPC_SELL: i16 = 3916;
    pub const REPURCHASE_ITEM: i16 = 3978;
    pub const LOAD_NPC_REPURCHASE_LIST: i16 = 3979;
}
