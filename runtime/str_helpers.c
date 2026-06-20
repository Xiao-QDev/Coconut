#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

void* coconut_read_file(const char* path) {
    FILE* f = fopen(path, "r");
    if (!f) {
        char* empty = (char*)malloc(1);
        if (empty) empty[0] = '\0';
        return empty;
    }
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    char* buf = (char*)malloc(size + 1);
    if (!buf) { fclose(f); return NULL; }
    size_t n = fread(buf, 1, size, f);
    buf[n] = '\0';
    fclose(f);
    return buf;
}

int coconut_write_file(const char* path, const char* content) {
    FILE* f = fopen(path, "w");
    if (!f) return -1;
    fputs(content, f);
    fclose(f);
    return 0;
}

void* coconut_substring(const char* s, int start, int len) {
    char* buf = (char*)malloc(len + 1);
    if (!buf) return NULL;
    int i;
    for (i = 0; i < len && s[start + i] != '\0'; i++) {
        buf[i] = s[start + i];
    }
    buf[i] = '\0';
    return buf;
}

void* coconut_append(const char* a, const char* b) {
    int la = strlen(a);
    int lb = strlen(b);
    char* buf = (char*)malloc(la + lb + 1);
    if (!buf) return NULL;
    memcpy(buf, a, la);
    memcpy(buf + la, b, lb + 1);
    return buf;
}

int coconut_str_equals(const char* a, const char* b) {
    return strcmp(a, b) == 0 ? 1 : 0;
}

int coconut_is_digit(int c) {
    return (c >= '0' && c <= '9') ? 1 : 0;
}

int coconut_is_alpha(int c) {
    return ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')) ? 1 : 0;
}

int coconut_is_space(int c) {
    return (c == ' ' || c == '\t' || c == '\n' || c == '\r') ? 1 : 0;
}

typedef struct {
    int64_t* data;
    int size;
    int cap;
} CIntArr;

void* coconut_dynarr_new(void) {
    CIntArr* arr = (CIntArr*)malloc(sizeof(CIntArr));
    if (!arr) return NULL;
    arr->cap = 8;
    arr->size = 0;
    arr->data = (int64_t*)malloc(sizeof(int64_t) * arr->cap);
    return arr;
}

void coconut_dynarr_push(void* h, int64_t val) {
    CIntArr* arr = (CIntArr*)h;
    if (!arr) return;
    if (arr->size >= arr->cap) {
        arr->cap *= 2;
        arr->data = (int64_t*)realloc(arr->data, sizeof(int64_t) * arr->cap);
    }
    arr->data[arr->size++] = val;
}

int64_t coconut_dynarr_get(void* h, int idx) {
    CIntArr* arr = (CIntArr*)h;
    if (!arr || idx < 0 || idx >= arr->size) return 0;
    return arr->data[idx];
}

void coconut_dynarr_set(void* h, int idx, int64_t val) {
    CIntArr* arr = (CIntArr*)h;
    if (!arr || idx < 0 || idx >= arr->size) return;
    arr->data[idx] = val;
}

int coconut_dynarr_len(void* h) {
    CIntArr* arr = (CIntArr*)h;
    return arr ? arr->size : 0;
}

typedef struct {
    char** data;
    int size;
    int cap;
} CStrArr;

void* coconut_strarr_new(void) {
    CStrArr* arr = (CStrArr*)malloc(sizeof(CStrArr));
    if (!arr) return NULL;
    arr->cap = 8;
    arr->size = 0;
    arr->data = (char**)malloc(sizeof(char*) * arr->cap);
    return arr;
}

void coconut_strarr_push(void* h, const char* s) {
    CStrArr* arr = (CStrArr*)h;
    if (!arr) return;
    if (arr->size >= arr->cap) {
        arr->cap *= 2;
        arr->data = (char**)realloc(arr->data, sizeof(char*) * arr->cap);
    }
    int len = strlen(s);
    arr->data[arr->size] = (char*)malloc(len + 1);
    memcpy(arr->data[arr->size], s, len + 1);
    arr->size++;
}

void* coconut_strarr_get(void* h, int idx) {
    CStrArr* arr = (CStrArr*)h;
    if (!arr || idx < 0 || idx >= arr->size) return "";
    return arr->data[idx];
}

int coconut_strarr_len(void* h) {
    CStrArr* arr = (CStrArr*)h;
    return arr ? arr->size : 0;
}

#define HASHMAP_SIZE 256

typedef struct HMNode {
    char* key;
    char* val;
    struct HMNode* next;
} HMNode;

typedef struct {
    HMNode* buckets[HASHMAP_SIZE];
} CHashMap;

static unsigned int _hm_hash(const char* s) {
    unsigned int h = 0;
    while (*s) { h = h * 31 + (unsigned char)*s++; }
    return h % HASHMAP_SIZE;
}

void* coconut_hashmap_new(void) {
    CHashMap* map = (CHashMap*)calloc(1, sizeof(CHashMap));
    return map;
}

void coconut_hashmap_set(void* h, const char* key, const char* val) {
    CHashMap* map = (CHashMap*)h;
    if (!map) return;
    unsigned int idx = _hm_hash(key);
    HMNode* node = map->buckets[idx];
    while (node) {
        if (strcmp(node->key, key) == 0) {
            free(node->val);
            node->val = (char*)malloc(strlen(val) + 1);
            strcpy(node->val, val);
            return;
        }
        node = node->next;
    }
    HMNode* nn = (HMNode*)malloc(sizeof(HMNode));
    nn->key = (char*)malloc(strlen(key) + 1);
    strcpy(nn->key, key);
    nn->val = (char*)malloc(strlen(val) + 1);
    strcpy(nn->val, val);
    nn->next = map->buckets[idx];
    map->buckets[idx] = nn;
}

void* coconut_hashmap_get(void* h, const char* key) {
    CHashMap* map = (CHashMap*)h;
    if (!map) return "";
    unsigned int idx = _hm_hash(key);
    HMNode* node = map->buckets[idx];
    while (node) {
        if (strcmp(node->key, key) == 0) return node->val;
        node = node->next;
    }
    return "";
}

int coconut_hashmap_has(void* h, const char* key) {
    CHashMap* map = (CHashMap*)h;
    if (!map) return 0;
    unsigned int idx = _hm_hash(key);
    HMNode* node = map->buckets[idx];
    while (node) {
        if (strcmp(node->key, key) == 0) return 1;
        node = node->next;
    }
    return 0;
}