/* libmelee: the shared application core for native hosts.
 *
 * The core owns the whole application: disc validation and reading, the
 * character/stage catalog, menu flow, loading, the match session and the
 * renderer. A host (the macOS app) draws the menus natively, forwards user
 * intent, and gives the core a surface to draw the game into.
 *
 * Threading: a melee_app_t is used from one thread (the main thread).
 * Ownership: strings returned in structs are static; the core copies every
 * string it is given and never retains host buffers.
 * Errors: calls returning bool report failure with false; the message is
 * then available from melee_app_last_error. */
#ifndef MELEE_PLATFORM_H
#define MELEE_PLATFORM_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

/* Bumped on any incompatible change; hosts check it at startup. */
#define MELEE_API_VERSION 2
uint32_t melee_api_version(void);

typedef struct melee_app_s melee_app_t;

typedef enum {
    MELEE_SCREEN_DISC = 0,       /* ask for the disc image */
    MELEE_SCREEN_CHARACTERS = 1, /* character select */
    MELEE_SCREEN_STAGES = 2,     /* stage select */
    MELEE_SCREEN_LOADING = 3,    /* call melee_app_load_step until done */
    MELEE_SCREEN_MATCH = 4,      /* call melee_app_frame every display frame */
    MELEE_SCREEN_RESULTS = 5,
} melee_screen_e;

typedef enum {
    MELEE_ACTION_LEFT = 0,
    MELEE_ACTION_RIGHT,
    MELEE_ACTION_UP,
    MELEE_ACTION_DOWN,
    MELEE_ACTION_ATTACK,
    MELEE_ACTION_SPECIAL,
    MELEE_ACTION_JUMP,
    MELEE_ACTION_SHIELD,
    MELEE_ACTION_GRAB,
} melee_action_e;

/* ---- Catalog: static, no app needed. Ids are indices. ---- */
typedef struct {
    const char *name;       /* display name, UTF-8, static */
    const char *key;        /* stable ASCII identifier, e.g. "CaptainFalcon" */
    uint8_t costume_count;
    uint8_t grid_row;       /* retail character select layout */
    uint8_t grid_column;
} melee_character_info_s;
typedef struct {
    const char *name;
    const char *key;        /* e.g. "FinalDestination" */
} melee_stage_info_s;
uint32_t melee_character_count(void);
bool melee_character_info(uint32_t id, melee_character_info_s *out);
uint32_t melee_stage_count(void);
bool melee_stage_info(uint32_t id, melee_stage_info_s *out);

/* ---- App ---- */
melee_app_t *melee_app_new(void);
void melee_app_free(melee_app_t *app);
melee_screen_e melee_app_screen(const melee_app_t *app);
/* Copies the last error into buf (NUL-terminated, truncated to capacity) and
 * returns its full length in bytes. */
size_t melee_app_last_error(const melee_app_t *app, char *buf, size_t capacity);
/* A message for the user (load failure, match fault with its replay path).
 * Returns its length and clears it; 0 when there is none. */
size_t melee_app_take_notice(melee_app_t *app, char *buf, size_t capacity);

/* ---- Disc ---- */
typedef struct {
    char game_id[8];
    char title[96];
    uint8_t revision;
    uint32_t cached_files;  /* files fetched so far, kept across matches */
    uint64_t cached_bytes;
} melee_disc_info_s;
/* Open and validate an image (Melee NTSC-U 1.02 .iso/.gcm). On success the
 * screen becomes MELEE_SCREEN_CHARACTERS and the menu art is read (about
 * 5 MB; a read failure only leaves the art missing). */
bool melee_app_open_disc(melee_app_t *app, const char *path);
bool melee_app_disc_info(const melee_app_t *app, melee_disc_info_s *out);
/* From the disc screen, continue with the open disc. */
bool melee_app_resume_disc(melee_app_t *app);

/* Files read so far out of a set: the menu art, or a match's files. */
typedef struct {
    uint32_t files_done;
    uint32_t files_total;
    uint64_t bytes_done;
    uint64_t bytes_total;
} melee_load_progress_s;

/* ---- Menu art, decoded from the open disc ----
 * Images are RGBA8, rows top to bottom, straight (unassociated) alpha, at
 * the texture's native size (never resampled; scale with nearest-neighbour
 * for pixel fidelity). Intensity images (emblems, stage names) carry the
 * intensity in every channel: use their alpha as a mask and tint it. */
typedef enum {
    MELEE_ART_PORTRAIT = 0,         /* character, costume: 136x188 select portrait */
    MELEE_ART_FACE = 1,             /* character: 64x56 select grid face with name */
    MELEE_ART_STOCK = 2,            /* character, costume: 24x24 stock icon */
    MELEE_ART_CHARACTER_EMBLEM = 3, /* character: 80x64 series emblem (intensity) */
    MELEE_ART_STAGE_ICON = 4,       /* stage: 64x56 icon (48x48 for Past Stages) */
    MELEE_ART_STAGE_NAME = 5,       /* stage: 224x56 name plate (intensity) */
    MELEE_ART_STAGE_EMBLEM = 6,     /* stage: 64x64 faint series watermark (intensity) */
} melee_art_e;
typedef struct {
    uint32_t width;
    uint32_t height;
    const uint8_t *rgba;    /* width * height * 4 bytes, owned by the core */
    size_t len;
} melee_image_s;
/* Whether every menu archive has been read. */
bool melee_app_art_ready(const melee_app_t *app);
void melee_app_art_progress(const melee_app_t *app, melee_load_progress_s *out);
/* Read the menu archives again after a failure. */
bool melee_app_load_art(melee_app_t *app);
/* One image. id is a character id for character kinds and a stage id for
 * stage kinds; costume is ignored where it does not apply. out->rgba stays
 * valid until the next melee_app_art call or melee_app_free; copy it to
 * keep it. Fails (see melee_app_last_error) before the art is read, for an
 * unknown id or costume, and where retail has no such image (Sheik has no
 * portrait or face: she is picked through Zelda's). */
bool melee_app_art(melee_app_t *app, melee_art_e kind, uint32_t id, uint8_t costume,
                   melee_image_s *out);

/* ---- Character select ---- */
typedef struct {
    int32_t character;  /* id, or -1 for none */
    uint8_t costume;
} melee_slot_s;
typedef struct {
    melee_slot_s players[2];
    uint8_t stocks;
    bool ready;         /* both players picked */
} melee_selection_s;
void melee_app_selection(const melee_app_t *app, melee_selection_s *out);
/* character -1 clears the pick. */
bool melee_app_choose_character(melee_app_t *app, uint32_t player, int32_t character);
bool melee_app_set_costume(melee_app_t *app, uint32_t player, uint8_t costume);
bool melee_app_cycle_costume(melee_app_t *app, uint32_t player, int32_t step);
bool melee_app_set_stocks(melee_app_t *app, uint8_t stocks);
bool melee_app_confirm_characters(melee_app_t *app);
/* One step back: stages -> characters -> disc; cancels a load; leaves results. */
void melee_app_back(melee_app_t *app);

/* ---- Stage select and loading ---- */
/* seed: any random number from the host; it is recorded in the replay. */
bool melee_app_choose_stage(melee_app_t *app, uint32_t stage, uint32_t seed);
void melee_app_load_progress(const melee_app_t *app, melee_load_progress_s *out);
/* Read the next file: 1 = more to read, 0 = all read, -1 = error (the screen
 * returns to stage select with a notice). */
int32_t melee_app_load_step(melee_app_t *app);
/* Build the match once all files are read. On failure the screen returns to
 * stage select and the notice says why. */
bool melee_app_finish_loading(melee_app_t *app);

/* ---- Surface ---- */
#ifdef __APPLE__
/* layer is a CAMetalLayer the host keeps alive until melee_app_detach_surface
 * or melee_app_free. Size is in pixels. */
bool melee_app_attach_metal_layer(melee_app_t *app, void *layer, uint32_t width, uint32_t height);
#endif
void melee_app_detach_surface(melee_app_t *app);

/* ---- Match ---- */
typedef struct {
    uint8_t port;       /* 0-based controller port */
    uint32_t character; /* catalog id */
    uint8_t costume;
    float percent;
    uint8_t stocks;
} melee_player_hud_s;
typedef struct {
    uint64_t tick;
    bool paused;
    bool faulted;       /* stopped on a fault; see the notice */
    melee_player_hud_s players[2];
} melee_hud_s;
typedef struct {
    int32_t winner;     /* player index 0/1, or -1 for a draw */
    melee_hud_s hud;    /* the final state */
} melee_results_s;
/* Advance by the host time since the previous frame and draw. size in pixels.
 * Returns false on a fault (the match stops; the notice carries the message). */
bool melee_app_frame(melee_app_t *app, uint32_t width, uint32_t height);
/* Whether the match needs frames (running, not paused). */
bool melee_app_needs_frame(const melee_app_t *app);
void melee_app_action(melee_app_t *app, uint32_t player, melee_action_e action, bool down);
void melee_app_set_paused(melee_app_t *app, bool paused);
void melee_app_set_focused(melee_app_t *app, bool focused);
bool melee_app_hud(const melee_app_t *app, melee_hud_s *out);
bool melee_app_results(const melee_app_t *app, melee_results_s *out);
bool melee_app_restart(melee_app_t *app);
/* Same characters and stage with a new seed: back to loading (instant). */
bool melee_app_rematch(melee_app_t *app, uint32_t seed);
void melee_app_quit_to_menu(melee_app_t *app);
/* Write the match's input recording (melee-replay JSON), replacing path. */
bool melee_app_save_replay(melee_app_t *app, const char *path);

#ifdef __cplusplus
}
#endif
#endif
