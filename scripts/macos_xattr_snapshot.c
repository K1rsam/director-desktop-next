/*
 * Read-only, bounded xattr collector for one absolute file or directory.
 * Build: cc -std=c11 -O2 -Wall -Wextra -Werror -pedantic -o collector this.c
 *
 * stdout is a private pipe protocol, NOT a report: successful nameHex/valueHex
 * fields contain lossless bytes and must be hashed before evidence is persisted.
 * No path, attribute name, attribute value, errno text, or partial snapshot is
 * included in an error. There is no attribute write or removal operation.
 *
 * This is a checked observation, not an atomic filesystem snapshot. Two complete
 * reads, descriptor metadata, and a second no-follow path lookup must agree.
 * A concurrent modification that is undone between observations cannot be ruled
 * out. The caller must separately guard the whole bundle/signing interval.
 * Darwin is the production platform; Linux supports isolated regression tests.
 */
#define _POSIX_C_SOURCE 200809L
#if defined(__APPLE__)
#define _DARWIN_C_SOURCE 1
#endif

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/xattr.h>
#include <unistd.h>

#if !defined(__APPLE__) && !defined(__linux__)
#error "The xattr collector supports Darwin and Linux only"
#endif

#define MAX_PATH_BYTES 32768U
#define MAX_NAMES_BYTES 65536U
#define MAX_ATTRIBUTES 64U
#define MAX_VALUE_BYTES 65536U
#define MAX_TOTAL_BYTES 262144U

struct attribute {
    const char *name;
    size_t name_size;
    unsigned char *value;
    size_t value_size;
};

struct snapshot {
    char *names;
    struct attribute attributes[MAX_ATTRIBUTES];
    size_t count;
    size_t names_bytes;
    size_t value_bytes;
};

/* Fixed, payload-free categories. Unknown errors always fail closed. */
static const char *classify_error(int error)
{
    if (error == ENOTSUP || error == EOPNOTSUPP || error == ENOSYS)
        return "XATTR_UNSUPPORTED";
    if (error == EACCES || error == EPERM)
        return "XATTR_PERMISSION_DENIED";
    if (error == ELOOP)
        return "XATTR_SYMLINK";
    if (error == ENOENT || error == ENOTDIR || error == ERANGE ||
        error == EINTR || error == ESTALE)
        return "XATTR_RACE";
#ifdef ENOATTR
    if (error == ENOATTR)
        return "XATTR_RACE";
#endif
#ifdef ENODATA
    if (error == ENODATA)
        return "XATTR_RACE";
#endif
    if (error == E2BIG || error == EFBIG || error == EOVERFLOW)
        return "XATTR_LIMIT";
    if (error == ENAMETOOLONG || error == EINVAL)
        return "XATTR_INVALID_ARGUMENT";
    if (error == ENOMEM || error == EMFILE || error == ENFILE)
        return "XATTR_RESOURCE_LIMIT";
    return "XATTR_IO_ERROR";
}

/* No symlink is followed, including in an ancestor or before a trailing '/'. */
static int open_absolute(const char *path, const char **error)
{
    size_t length = strnlen(path, MAX_PATH_BYTES + 1U);
    char *copy;
    char *component;
    int descriptor;

    if (length == 0U || path[0] != '/' || length > MAX_PATH_BYTES) {
        *error = "XATTR_INVALID_ARGUMENT";
        return -1;
    }
    copy = malloc(length + 1U);
    if (copy == NULL) {
        *error = "XATTR_RESOURCE_LIMIT";
        return -1;
    }
    memcpy(copy, path, length + 1U);
    descriptor = open("/", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (descriptor == -1) {
        *error = classify_error(errno);
        free(copy);
        return -1;
    }
    component = copy + 1;
    while (*component != '\0') {
        char *next;
        char *end;
        int next_descriptor;
        int flags = O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK;
        struct stat entry;

        while (*component == '/')
            ++component;
        if (*component == '\0')
            break;
        end = strchr(component, '/');
        next = end == NULL ? component + strlen(component) : end + 1;
        if (end != NULL)
            *end = '\0';
        while (*next == '/')
            ++next;
        if (strcmp(component, ".") == 0 || strcmp(component, "..") == 0) {
            *error = "XATTR_INVALID_ARGUMENT";
            goto failed;
        }
        /* Classify links explicitly: O_NOFOLLOW|O_DIRECTORY can yield ENOTDIR. */
        if (fstatat(descriptor, component, &entry, AT_SYMLINK_NOFOLLOW) == -1) {
            *error = classify_error(errno);
            goto failed;
        }
        if (S_ISLNK(entry.st_mode)) {
            *error = "XATTR_SYMLINK";
            goto failed;
        }
        if (end != NULL)
            flags |= O_DIRECTORY;
        next_descriptor = openat(descriptor, component, flags);
        if (next_descriptor == -1) {
            *error = classify_error(errno);
            goto failed;
        }
        if (close(descriptor) == -1) {
            *error = classify_error(errno);
            (void)close(next_descriptor);
            free(copy);
            return -1;
        }
        descriptor = next_descriptor;
        component = next;
    }
    free(copy);
    return descriptor;

failed:
    (void)close(descriptor);
    free(copy);
    return -1;
}

static ssize_t read_names(int descriptor, char *names, size_t size)
{
#if defined(__APPLE__)
    /* Default Darwin enumeration hides compression metadata. Include it. */
    return flistxattr(descriptor, names, size, XATTR_SHOWCOMPRESSION);
#else
    return flistxattr(descriptor, names, size);
#endif
}

static ssize_t read_value(int descriptor, const char *name, void *value, size_t size)
{
#if defined(__APPLE__)
    return fgetxattr(descriptor, name, value, size, 0, XATTR_SHOWCOMPRESSION);
#else
    return fgetxattr(descriptor, name, value, size);
#endif
}

static int compare_names(const void *left, const void *right)
{
    const struct attribute *a = left;
    const struct attribute *b = right;
    return strcmp(a->name, b->name);
}

static void free_snapshot(struct snapshot *snapshot)
{
    size_t index;
    for (index = 0; index < snapshot->count; ++index)
        free(snapshot->attributes[index].value);
    free(snapshot->names);
}

static const char *collect(int descriptor, struct snapshot *snapshot)
{
    ssize_t expected = read_names(descriptor, NULL, 0);
    ssize_t actual;
    size_t offset = 0;
    size_t index;

    if (expected < 0)
        return classify_error(errno);
    if ((size_t)expected > MAX_NAMES_BYTES)
        return "XATTR_NAMES_LIMIT";
    snapshot->names_bytes = (size_t)expected;
    snapshot->names = malloc(expected == 0 ? 1U : (size_t)expected);
    if (snapshot->names == NULL)
        return "XATTR_RESOURCE_LIMIT";
    /* A nonzero buffer detects a change even when the initial list was empty. */
    actual = read_names(descriptor, snapshot->names,
                        expected == 0 ? 1U : (size_t)expected);
    if (actual < 0)
        return classify_error(errno);
    if (actual != expected)
        return "XATTR_RACE";
    while (offset < (size_t)actual) {
        const char *name = snapshot->names + offset;
        const char *end = memchr(name, '\0', (size_t)actual - offset);
        struct attribute *attribute;
        if (end == NULL || end == name)
            return "XATTR_MALFORMED";
        if (snapshot->count == MAX_ATTRIBUTES)
            return "XATTR_COUNT_LIMIT";
        attribute = &snapshot->attributes[snapshot->count++];
        attribute->name = name;
        attribute->name_size = (size_t)(end - name);
        offset += attribute->name_size + 1U;
    }
    qsort(snapshot->attributes, snapshot->count,
          sizeof(snapshot->attributes[0]), compare_names);
    for (index = 0; index < snapshot->count; ++index) {
        struct attribute *attribute = &snapshot->attributes[index];
        if (index > 0 && strcmp(snapshot->attributes[index - 1U].name,
                                attribute->name) == 0)
            return "XATTR_MALFORMED";
        expected = read_value(descriptor, attribute->name, NULL, 0);
        if (expected < 0)
            return classify_error(errno);
        if ((size_t)expected > MAX_VALUE_BYTES)
            return "XATTR_VALUE_LIMIT";
        if ((size_t)expected > MAX_TOTAL_BYTES - snapshot->names_bytes - snapshot->value_bytes)
            return "XATTR_TOTAL_LIMIT";
        attribute->value = malloc(expected == 0 ? 1U : (size_t)expected);
        if (attribute->value == NULL)
            return "XATTR_RESOURCE_LIMIT";
        actual = read_value(descriptor, attribute->name, attribute->value,
                            expected == 0 ? 1U : (size_t)expected);
        if (actual < 0)
            return classify_error(errno);
        if (actual != expected)
            return "XATTR_RACE";
        attribute->value_size = (size_t)actual;
        snapshot->value_bytes += attribute->value_size;
    }
    return NULL;
}

static int same_metadata(const struct stat *a, const struct stat *b)
{
    if (a->st_dev != b->st_dev || a->st_ino != b->st_ino ||
        a->st_mode != b->st_mode || a->st_uid != b->st_uid ||
        a->st_gid != b->st_gid || a->st_nlink != b->st_nlink ||
        a->st_size != b->st_size)
        return 0;
#if defined(__APPLE__)
    return a->st_mtimespec.tv_sec == b->st_mtimespec.tv_sec &&
           a->st_mtimespec.tv_nsec == b->st_mtimespec.tv_nsec &&
           a->st_ctimespec.tv_sec == b->st_ctimespec.tv_sec &&
           a->st_ctimespec.tv_nsec == b->st_ctimespec.tv_nsec &&
           a->st_birthtimespec.tv_sec == b->st_birthtimespec.tv_sec &&
           a->st_birthtimespec.tv_nsec == b->st_birthtimespec.tv_nsec &&
           a->st_flags == b->st_flags && a->st_gen == b->st_gen;
#else
    return a->st_mtim.tv_sec == b->st_mtim.tv_sec &&
           a->st_mtim.tv_nsec == b->st_mtim.tv_nsec &&
           a->st_ctim.tv_sec == b->st_ctim.tv_sec &&
           a->st_ctim.tv_nsec == b->st_ctim.tv_nsec;
#endif
}

static int same_snapshot(const struct snapshot *a, const struct snapshot *b)
{
    size_t index;
    if (a->count != b->count || a->value_bytes != b->value_bytes)
        return 0;
    for (index = 0; index < a->count; ++index) {
        const struct attribute *left = &a->attributes[index];
        const struct attribute *right = &b->attributes[index];
        if (left->name_size != right->name_size ||
            memcmp(left->name, right->name, left->name_size) != 0 ||
            left->value_size != right->value_size ||
            memcmp(left->value, right->value, left->value_size) != 0)
            return 0;
    }
    return 1;
}

static void print_hex(const unsigned char *bytes, size_t size)
{
    static const char digits[] = "0123456789abcdef";
    size_t index;
    for (index = 0; index < size; ++index) {
        (void)putchar(digits[bytes[index] >> 4]);
        (void)putchar(digits[bytes[index] & 15U]);
    }
}

int main(int argc, char **argv)
{
    const char *error = NULL;
    int descriptor = -1;
    int check_descriptor = -1;
    struct stat before;
    struct stat after;
    struct stat path_after;
    struct snapshot first = {0};
    struct snapshot second = {0};
    size_t index;

    if (argc != 2) {
        error = "XATTR_INVALID_ARGUMENT";
        goto finished;
    }
    descriptor = open_absolute(argv[1], &error);
    if (descriptor == -1)
        goto finished;
    if (fstat(descriptor, &before) == -1) {
        error = classify_error(errno);
        goto finished;
    }
    if (!S_ISREG(before.st_mode) && !S_ISDIR(before.st_mode)) {
        error = "XATTR_KIND_UNSUPPORTED";
        goto finished;
    }
    error = collect(descriptor, &first);
    if (error != NULL)
        goto finished;
    error = collect(descriptor, &second);
    if (error != NULL)
        goto finished;
    if (!same_snapshot(&first, &second)) {
        error = "XATTR_RACE";
        goto finished;
    }
    if (fstat(descriptor, &after) == -1) {
        error = classify_error(errno);
        goto finished;
    }
    check_descriptor = open_absolute(argv[1], &error);
    if (check_descriptor == -1)
        goto finished;
    if (fstat(check_descriptor, &path_after) == -1) {
        error = classify_error(errno);
        goto finished;
    }
    if (!same_metadata(&before, &after) || !same_metadata(&after, &path_after))
        error = "XATTR_RACE";

finished:
    if (descriptor != -1 && close(descriptor) == -1 && error == NULL)
        error = classify_error(errno);
    if (check_descriptor != -1 && close(check_descriptor) == -1 && error == NULL)
        error = classify_error(errno);
    if (error != NULL) {
        (void)printf("{\"schemaVersion\":1,\"ok\":false,\"error\":\"%s\"}\n", error);
    } else {
        (void)printf("{\"schemaVersion\":1,\"ok\":true,\"kind\":\"%s\",\"attributes\":[",
                     S_ISDIR(before.st_mode) ? "directory" : "file");
        for (index = 0; index < first.count; ++index) {
            const struct attribute *attribute = &first.attributes[index];
            (void)printf("%s{\"nameHex\":\"", index == 0 ? "" : ",");
            print_hex((const unsigned char *)attribute->name, attribute->name_size);
            (void)fputs("\",\"valueHex\":\"", stdout);
            print_hex(attribute->value, attribute->value_size);
            (void)fputs("\"}", stdout);
        }
        (void)fputs("]}\n", stdout);
    }
    free_snapshot(&second);
    free_snapshot(&first);
    /* Output errors never turn a failed/private partial transfer into success. */
    if (fflush(stdout) != 0 || ferror(stdout))
        return 1;
    return error == NULL ? 0 : 1;
}
