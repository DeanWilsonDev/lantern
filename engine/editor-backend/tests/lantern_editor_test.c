#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "lantern_editor.h"

static int failureCount = 0;
static const char* assetsDirectory = NULL;

#define CHECK(condition)                                                        \
    do {                                                                        \
        if (!(condition)) {                                                     \
            fprintf(stderr, "%s:%d: check failed: %s\n", __FILE__, __LINE__, #condition); \
            ++failureCount;                                                     \
        }                                                                       \
    } while (0)

#define CHECK_TEXT(actual, expected)                                            \
    do {                                                                        \
        const char* actualText = (actual);                                      \
        if (strcmp(actualText, (expected)) != 0) {                              \
            fprintf(stderr, "%s:%d: '%s' is \"%s\", expected \"%s\"\n", __FILE__, __LINE__, #actual, actualText, \
                    (expected));                                                \
            ++failureCount;                                                     \
        }                                                                       \
    } while (0)

static bool TextContains(const char* text, const char* part) {
    return strstr(text, part) != NULL;
}

static bool TextStartsWith(const char* text, const char* prefix) {
    return strncmp(text, prefix, strlen(prefix)) == 0;
}

static bool TextEndsWith(const char* text, const char* suffix) {
    size_t textLength = strlen(text);
    size_t suffixLength = strlen(suffix);
    return textLength >= suffixLength && strcmp(text + textLength - suffixLength, suffix) == 0;
}

static void PanelsThroughTheCApi(void) {
    LanternEditor* editor = lantern_editor_create(assetsDirectory, NULL);
    CHECK(editor != NULL);
    if (!editor) return;
    CHECK_TEXT(lantern_editor_title(editor), "course.world.json");
    CHECK(lantern_editor_outline_count(editor) == 29);
    int64_t first = lantern_editor_outline_id(editor, 0);
    CHECK_TEXT(lantern_editor_entity_name(editor, first), "start_floor");
    CHECK(lantern_editor_field_count(editor) == 0);

    uint64_t revision = lantern_editor_revision(editor);
    lantern_editor_select(editor, first, false);
    CHECK(lantern_editor_revision(editor) > revision);
    CHECK(lantern_editor_primary_selection(editor) == first);

    uint32_t fieldCount = lantern_editor_field_count(editor);
    bool positionFound = false;
    uint32_t materialIndex = fieldCount;
    for (uint32_t index = 0; index < fieldCount; ++index) {
        char key[128];
        snprintf(key, sizeof key, "%s", lantern_editor_field(editor, index, LANTERN_FIELD_KEY));
        if (strcmp(key, "position.y") == 0 && strcmp(lantern_editor_field(editor, index, LANTERN_FIELD_VALUE), "-0.5") == 0) {
            positionFound = true;
        }
        if (strcmp(key, "material") == 0) {
            materialIndex = index;
        }
    }
    CHECK(positionFound);
    CHECK(materialIndex < fieldCount);
    CHECK_TEXT(lantern_editor_field(editor, materialIndex, LANTERN_FIELD_KIND), "choice");
    CHECK(lantern_editor_field_choice_count(editor, materialIndex) >= 3);

    CHECK(!lantern_editor_set_field(editor, first, "position.y", "down"));
    CHECK(TextContains(lantern_editor_status(editor), "not a number"));
    CHECK(lantern_editor_set_field(editor, first, "position.y", "-2"));
    CHECK(lantern_editor_is_dirty(editor));
    CHECK(lantern_editor_undo(editor));
    CHECK(!lantern_editor_is_dirty(editor));

    CHECK_TEXT(lantern_editor_template(editor, 0, LANTERN_TEMPLATE_LABEL), "Block");
    CHECK(lantern_editor_add_entity(editor, "block") > 0);
    CHECK(lantern_editor_outline_count(editor) == 30);

    lantern_editor_set_tool(editor, "rotate");
    CHECK_TEXT(lantern_editor_tool(editor), "rotate");
    CHECK(lantern_editor_key(editor, "w", true, 0));
    CHECK_TEXT(lantern_editor_tool(editor), "move");
    CHECK(lantern_editor_delete_selection(editor));
    CHECK(lantern_editor_outline_count(editor) == 29);
    lantern_editor_destroy(editor);
}

static void TheToolsThroughTheCApi(void) {
    LanternEditor* editor = lantern_editor_create(assetsDirectory, NULL);
    CHECK(editor != NULL);
    if (!editor) return;
    CHECK(lantern_editor_entity_count(editor) == 29);
    int64_t first = lantern_editor_outline_id(editor, 0);
    CHECK(lantern_editor_outline_parent(editor, 0) == 0);
    lantern_editor_set_hidden(editor, first, true);
    CHECK(lantern_editor_is_hidden(editor, first));
    lantern_editor_unhide_all(editor);
    CHECK(!lantern_editor_is_hidden(editor, first));

    lantern_editor_set_space(editor, "local");
    CHECK_TEXT(lantern_editor_space(editor), "local");
    lantern_editor_set_pivot_mode(editor, "center");
    CHECK_TEXT(lantern_editor_pivot_mode(editor), "center");
    CHECK(lantern_editor_set_snap_step(editor, "translate", 0.5f));
    CHECK(lantern_editor_snap_step(editor, "translate") == 0.5f);
    CHECK(lantern_editor_set_view_option(editor, "wireframe", "true"));
    CHECK_TEXT(lantern_editor_view_option(editor, "wireframe"), "true");
    CHECK(!lantern_editor_set_view_option(editor, "nonsense", "true"));

    CHECK(lantern_editor_camera_preset(editor, "top"));
    CHECK(lantern_editor_is_orthographic(editor));
    CHECK(TextStartsWith(lantern_editor_camera_description(editor), "ortho"));
    CHECK(lantern_editor_set_bookmark(editor, 0));
    CHECK(lantern_editor_has_bookmark(editor, 0));
    CHECK(!lantern_editor_has_bookmark(editor, 1));

    lantern_editor_select(editor, first, false);
    int64_t second = lantern_editor_outline_id(editor, 1);
    lantern_editor_select(editor, second, true);
    CHECK(lantern_editor_selection_count(editor) == 2);
    CHECK(lantern_editor_set_selection_field(editor, "property:speed", "2"));
    CHECK_TEXT(lantern_editor_field(editor, lantern_editor_field_count(editor) - 1, LANTERN_FIELD_MIXED), "false");
    CHECK(lantern_editor_group_selection(editor));
    CHECK(lantern_editor_entity_count(editor) == 30);
    CHECK(lantern_editor_history_count(editor) == 2);
    CHECK(lantern_editor_history_position(editor) == 2);
    CHECK_TEXT(lantern_editor_history_label(editor, 0), "Edit speed on 2 entities");
    CHECK(lantern_editor_jump_history(editor, 0));
    CHECK(lantern_editor_entity_count(editor) == 29);
    CHECK(lantern_editor_history_count(editor) == 2);

    CHECK(lantern_editor_log_count(editor) > 0);
    CHECK_TEXT(lantern_editor_log_level(editor, 0), "info");
    lantern_editor_clear_log(editor);
    CHECK(lantern_editor_log_count(editor) == 0);

    CHECK(lantern_editor_set_preference(editor, "window.width", "1600"));
    CHECK_TEXT(lantern_editor_preference(editor, "window.width"), "1600");
    CHECK_TEXT(lantern_editor_stat(editor, "entities"), "29");
    CHECK(!lantern_editor_is_playing(editor));
    CHECK(TextEndsWith(lantern_editor_assets_directory(editor), "assets"));
    lantern_editor_destroy(editor);
}

static bool WritePortablePixmap(const char* path, const uint8_t* pixels, uint32_t width, uint32_t height) {
    FILE* file = fopen(path, "wb");
    if (!file) return false;
    fprintf(file, "P6\n%u %u\n255\n", width, height);
    for (size_t index = 0; index < (size_t)width * height; ++index) {
        fwrite(pixels + index * 4, 1, 3, file);
    }
    fclose(file);
    return true;
}

static void TheViewportRendersTheCourse(void) {
    const char* requireText = getenv("LANTERN_REQUIRE_GPU");
    bool gpuRequired = requireText && strcmp(requireText, "1") == 0;
    LanternEditor* editor = lantern_editor_create(assetsDirectory, NULL);
    CHECK(editor != NULL);
    if (!editor) return;
    if (!lantern_editor_render(editor, 640, 360)) {
        const char* reason = lantern_editor_status(editor);
        if (gpuRequired) {
            fprintf(stderr, "GPU required: %s\n", reason);
            ++failureCount;
        } else {
            fprintf(stderr, "skipping viewport render: %s\n", reason);
        }
        lantern_editor_destroy(editor);
        return;
    }
    int64_t pillar = 0;
    for (uint32_t row = 0; row < lantern_editor_outline_count(editor); ++row) {
        int64_t entity = lantern_editor_outline_id(editor, row);
        if (strcmp(lantern_editor_entity_name(editor, entity), "pillar_2") == 0) {
            pillar = entity;
        }
    }
    CHECK(pillar != 0);
    lantern_editor_select(editor, pillar, false);
    CHECK(lantern_editor_render(editor, 640, 360));
    uint32_t width = 0;
    uint32_t height = 0;
    const uint8_t* pixels = lantern_editor_pixels(editor, &width, &height);
    CHECK(pixels != NULL);
    CHECK(width == 640 && height == 360);
    if (pixels && width == 640 && height == 360) {
        size_t differentCount = 0;
        for (size_t index = 0; index < (size_t)width * height; ++index) {
            if (memcmp(pixels + index * 4, pixels, 4) != 0) ++differentCount;
        }
        if (differentCount <= (size_t)width * height / 5) {
            fprintf(stderr, "only %zu pixels differ from the sky\n", differentCount);
            ++failureCount;
        }
        const char* capturePath = getenv("LANTERN_EDITOR_CAPTURE");
        if (capturePath && capturePath[0]) {
            CHECK(WritePortablePixmap(capturePath, pixels, width, height));
        }
    }

    CHECK(lantern_editor_render_preview(editor, "block", 96, 64, 0.3f));
    uint32_t previewWidth = 0;
    uint32_t previewHeight = 0;
    CHECK(lantern_editor_preview_pixels(editor, &previewWidth, &previewHeight) != NULL);
    CHECK(previewWidth == 96 && previewHeight == 64);
    uint32_t thumbnailSide = 0;
    CHECK(lantern_editor_thumbnail(editor, "block", 48, &thumbnailSide) != NULL);
    CHECK(thumbnailSide == 48);
    CHECK(lantern_editor_thumbnail(editor, "no-such-template", 48, &thumbnailSide) == NULL);
    CHECK(thumbnailSide == 0);
    lantern_editor_destroy(editor);
}

int main(int argumentCount, char** arguments) {
    if (argumentCount < 2) {
        fprintf(stderr, "usage: %s ASSETS_DIRECTORY\n", arguments[0]);
        return 2;
    }
    assetsDirectory = arguments[1];
    PanelsThroughTheCApi();
    TheToolsThroughTheCApi();
    TheViewportRendersTheCourse();
    CHECK(lantern_editor_title(NULL)[0] == '\0');
    CHECK(lantern_editor_pixels(NULL, NULL, NULL) == NULL);
    if (failureCount != 0) {
        fprintf(stderr, "%d checks failed\n", failureCount);
        return 1;
    }
    printf("the editor backend works through its C API\n");
    return 0;
}
