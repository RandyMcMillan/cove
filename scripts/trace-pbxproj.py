#!/usr/bin/env python3
"""Trace brace/paren/bracket depth through a project.pbxproj."""
import sys


def tokenize(content):
    i = 0
    n = len(content)
    tokens = []
    in_string = False
    escape = False
    in_comment = False

    while i < n:
        c = content[i]
        if in_comment:
            if c == '*' and i + 1 < n and content[i + 1] == '/':
                in_comment = False
                i += 2
                continue
            i += 1
            continue
        if in_string:
            if escape:
                escape = False
            elif c == '\\':
                escape = True
            elif c == '"':
                in_string = False
            i += 1
            continue
        if c == '"':
            in_string = True
            i += 1
            continue
        if c == '/' and i + 1 < n and content[i + 1] == '*':
            in_comment = True
            i += 2
            continue
        if c == '/' and i + 1 < n and content[i + 1] == '/':
            while i < n and content[i] != '\n':
                i += 1
            continue
        if c in '({[)}]':
            tokens.append((c, content[:i].count('\n') + 1))
        i += 1

    return tokens


def main(path):
    with open(path, 'r') as f:
        content = f.read()

    tokens = tokenize(content)
    stack = []
    for idx, (c, line) in enumerate(tokens):
        if c in '({[':
            stack.append((c, line))
        else:
            if not stack:
                print(f'EMPTY at token {idx}/{len(tokens)}: {c}@{line}')
            else:
                stack.pop()
        if 480 <= idx <= 520 or idx >= 590:
            print(f'token {idx}: {c}@{line} depth={len(stack)}')

    print(f'Total tokens: {len(tokens)}')
    print(f'Final stack depth: {len(stack)}')
    if stack:
        for opener, line in stack[-10:]:
            print(f'  unclosed {opener}@{line}')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        print(f'usage: {sys.argv[0]} <project.pbxproj>', file=sys.stderr)
        sys.exit(2)
    main(sys.argv[1])
