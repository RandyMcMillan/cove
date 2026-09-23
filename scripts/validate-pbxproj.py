#!/usr/bin/env python3
"""Validate an Xcode project.pbxproj for basic syntax and balance."""
import sys


def main(path):
    with open(path, 'r') as f:
        content = f.read()

    ok = True

    # Conflict markers
    for marker in ('<<<<<<<', '=======', '>>>>>>>'):
        if marker in content:
            print(f'CONFLICT MARKER: {marker}')
            ok = False

    # Section marker pairs
    sections = [
        'PBXBuildFile', 'PBXContainerItemProxy', 'PBXFileSystemSynchronizedBuildFileExceptionSet',
        'PBXFileSystemSynchronizedRootGroup', 'PBXFrameworksBuildPhase', 'PBXGroup',
        'PBXLegacyTarget', 'PBXNativeTarget', 'PBXProject', 'PBXResourcesBuildPhase',
        'PBXSourcesBuildPhase', 'PBXTargetDependency', 'PBXVariantGroup',
        'XCBuildConfiguration', 'XCConfigurationList', 'XCSwiftPackageProductDependency'
    ]
    for sec in sections:
        b = content.count(f'/* Begin {sec} section */')
        e = content.count(f'/* End {sec} section */')
        if b != e:
            print(f'SECTION MISMATCH: {sec}: begin={b}, end={e}')
            ok = False

    # Brace/paren/bracket balance ignoring strings and comments
    i = 0
    n = len(content)
    stack = []
    in_string = False
    escape = False
    in_comment = False
    pairs = {'(': ')', '{': '}', '[': ']'}

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
        if c in '({[':
            stack.append((c, content[:i].count('\n') + 1))
        elif c in ')}]':
            if not stack:
                print(f'UNEXPECTED {c} at line {content[:i].count(chr(10)) + 1}')
                ok = False
            else:
                opener, oline = stack.pop()
                if pairs[opener] != c:
                    print(f'MISMATCHED {opener} (line {oline}) and {c} (line {content[:i].count(chr(10)) + 1})')
                    ok = False
        i += 1

    if stack:
        for opener, line in stack:
            print(f'UNCLOSED {opener} at line {line}')
        ok = False

    if in_string:
        print('UNTERMINATED STRING')
        ok = False

    if ok:
        print('OK: no conflict markers, balanced braces/parens/brackets, section markers match')
    else:
        sys.exit(1)


if __name__ == '__main__':
    if len(sys.argv) != 2:
        print(f'usage: {sys.argv[0]} <project.pbxproj>', file=sys.stderr)
        sys.exit(2)
    main(sys.argv[1])
