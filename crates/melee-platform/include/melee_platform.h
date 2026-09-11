#ifndef MELEE_PLATFORM_H
#define MELEE_PLATFORM_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* All calls for a handle must be serialized. Destroy it before its native layer.
   The library copies the directory string. It never retains output buffers. */
typedef struct MeleeSession MeleeSession;
typedef struct { uint64_t tick; float damage[2]; float position[2][2]; uint8_t stocks[2]; uint8_t running; uint8_t paused; uint8_t needs_frame; } MeleeStatus;
typedef enum { MELEE_LEFT, MELEE_RIGHT, MELEE_UP, MELEE_DOWN, MELEE_ATTACK,
    MELEE_SPECIAL, MELEE_JUMP, MELEE_SHIELD, MELEE_GRAB } MeleeAction;
MeleeSession *melee_session_create(const char *directory, char *error, size_t capacity);
void melee_session_destroy(MeleeSession *session);
void melee_session_action(MeleeSession *session, uint32_t player, uint32_t action, bool down);
void melee_session_focus(MeleeSession *session, bool focused);
void melee_session_toggle_pause(MeleeSession *session);
void melee_session_pause(MeleeSession *session, bool paused);
bool melee_session_reset(MeleeSession *session);
bool melee_session_frame(MeleeSession *session, uint32_t width, uint32_t height);
bool melee_session_status(const MeleeSession *session, MeleeStatus *status);
void melee_session_error(const MeleeSession *session, char *error, size_t capacity);
#ifdef __APPLE__
/* layer is a CAMetalLayer; caller retains it until session destruction. */
bool melee_session_attach_macos(MeleeSession *session, void *layer, uint32_t width, uint32_t height);
#endif
#ifdef __cplusplus
}
#endif
#endif
