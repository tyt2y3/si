load(
    "@prelude-si//:cargo.bzl",
    _cargo_clippy_fix = "cargo_clippy_fix",
    _cargo_doc = "cargo_doc",
    _cargo_doc_check = "cargo_doc_check",
    _cargo_fmt = "cargo_fmt",
    _cargo_fmt_check = "cargo_fmt_check",
)
load(
    "@prelude-si//:rust.bzl",
    _clippy_check = "clippy_check",
)
load(
    "@prelude-si//macros:native.bzl",
    _alias = "alias",
    _test_suite = "test_suite",
)

def rust_binary(
        name,
        srcs,
        deps,
        edition = "2021",
        resources = [],
        test_unit_deps = [],
        test_unit_srcs = [],
        test_unit_resources = {},
        extra_test_targets = [],
        visibility = ["PUBLIC"],
        **kwargs):

    native.rust_binary(
        name = name,
        edition = edition,
        srcs = srcs,
        deps = deps,
        resources = resources,
        visibility = visibility,
        **kwargs
    )

    _alias(
        name = "build",
        actual = ":{}".format(name),
    )

    native.rust_test(
        name = "test-unit",
        edition = edition,
        srcs = srcs + test_unit_srcs,
        deps = deps + test_unit_deps,
        resources = test_unit_resources,
        visibility = visibility,
        **kwargs
    )

    _test_suite(
        name = "test",
        tests = [":test-unit"] + extra_test_targets,
        visibility = visibility,
    )

    _cargo_doc_check(
        name = "check-doc",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

    _cargo_fmt_check(
        name = "check-format",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

    _clippy_check(
        name = "check-lint-bin",
        clippy_txt_dep = ":{}[clippy.txt]".format(name),
        visibility = visibility,
    )

    _clippy_check(
        name = "check-lint-unit",
        clippy_txt_dep = ":{}[clippy.txt]".format("test-unit"),
        visibility = visibility,
    )

    extra_check_lint_targets = []
    for extra_test_target in extra_test_targets:
        check_name = "check-lint-{}".format(extra_test_target.replace("test-", ""))
        _clippy_check(
            name = check_name,
            clippy_txt_dep = "{}[clippy.txt]".format(extra_test_target),
            visibility = visibility,
        )
        extra_check_lint_targets.append(":{}".format(check_name))

    _test_suite(
        name = "check-lint",
        tests = [
            ":check-lint-bin",
            ":check-lint-unit",
        ] + extra_check_lint_targets,
        visibility = visibility,
    )

    _test_suite(
        name = "check",
        tests = [
            ":check-doc",
            ":check-format",
            ":check-lint-bin",
        ],
        visibility = visibility,
    )

    _cargo_fmt(
        name = "fix-format",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

    _cargo_clippy_fix(
        name = "fix-lint",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

    _cargo_doc(
        name = "doc",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

def rust_library(
        name,
        srcs,
        deps,
        edition = "2021",
        resources = [],
        test_unit_deps = [],
        test_unit_srcs = [],
        test_unit_resources = {},
        extra_test_targets = [],
        proc_macro = False,
        visibility = ["PUBLIC"],
        **kwargs):

    native.rust_library(
        name = name,
        edition = edition,
        srcs = srcs,
        deps = deps,
        resources = resources,
        proc_macro = proc_macro,
        visibility = visibility,
        **kwargs
    )

    _alias(
        name = "build",
        actual = ":{}".format(name),
    )

    native.rust_test(
        name = "test-unit",
        edition = edition,
        srcs = srcs + test_unit_srcs,
        deps = deps + test_unit_deps,
        resources = test_unit_resources,
        visibility = visibility,
        **kwargs
    )

    _test_suite(
        name = "test",
        tests = [":test-unit"] + extra_test_targets,
        visibility = visibility,
    )

    _cargo_doc_check(
        name = "check-doc",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

    _cargo_fmt_check(
        name = "check-format",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

    _clippy_check(
        name = "check-lint-lib",
        clippy_txt_dep = ":{}[clippy.txt]".format(name),
        visibility = visibility,
    )

    _clippy_check(
        name = "check-lint-unit",
        clippy_txt_dep = ":{}[clippy.txt]".format("test-unit"),
        visibility = visibility,
    )

    extra_check_lint_targets = []
    for extra_test_target in extra_test_targets:
        check_name = "check-lint-{}".format(extra_test_target.replace(":", "").replace("test-", ""))
        _clippy_check(
            name = check_name,
            clippy_txt_dep = "{}[clippy.txt]".format(extra_test_target),
            visibility = visibility,
        )
        extra_check_lint_targets.append(":{}".format(check_name))

    _test_suite(
        name = "check-lint",
        tests = [
            ":check-lint-lib",
            ":check-lint-unit",
        ] + extra_check_lint_targets,
        visibility = visibility,
    )

    _test_suite(
        name = "check",
        tests = [
            ":check-doc",
            ":check-format",
            ":check-lint-lib",
        ],
        visibility = visibility,
    )

    _cargo_fmt(
        name = "fix-format",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

    _cargo_clippy_fix(
        name = "fix-lint",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

    _cargo_doc(
        name = "doc",
        crate = name,
        srcs = srcs,
        visibility = visibility,
    )

def rust_test(
        name,
        edition = "2021",
        visibility = ["PUBLIC"],
        **kwargs):

    native.rust_test(
        name = name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )
