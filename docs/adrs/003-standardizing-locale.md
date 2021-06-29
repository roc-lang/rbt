# ADR 003: Standardizing Locale

Decision: set `LC_ALL=C.UTF-8` in builds to try to improve default reproducibility.
Allow it to be overriden if provided by a job.

## Background and Motivation

Environment variables like `LC` get set by the system language.
For example, on my computer, `locale` shows:

```
$ locale
LANG="en_US.UTF-8"
LC_COLLATE="en_US.UTF-8"
LC_CTYPE="en_US.UTF-8"
LC_MESSAGES="en_US.UTF-8"
LC_MONETARY="en_US.UTF-8"
LC_NUMERIC="en_US.UTF-8"
LC_TIME="en_US.UTF-8"
LC_ALL=
```

Among other things, that means that software that cares about locales will show numbers like "3.14" instead of "3,14" and use "$" as a prefix to talk about money.
This is great for me!

But, it also means that if I build some software that uses these settings at compile time without controlling for locale variables, it'll change from computer to computer.
Oh no, a source of irreproducibility!

### Consequences of setting `LC_ALL=C.UTF-8`

Looking at the diff in appendix A below, it appears that setting `LC_ALL=C.UTF-8` would:

- Set the charmap to `US-ASCII`.
  We almost certainly don't want this!
  I don't know why it's doing this!
  We probably want the charmap to be `UTF-8` if we're going to set it.
- Remove locale-specific currency handling.
- Set a consistent date format.
  The day and month in this output are in English (e.g. `LC_ALL=C.UTF-8 date '+%B'` says "June")
  It also has a two-digit year format for dates.
- Standardize locale-specific digit groupings.
  For example, it appears that "1,234.56" is the same in both `C.UTF-8` and `en_US.UTF-8`.
  (However, it's `1234.56` in `C`.
  Maybe that's superior here?)
- Confirmation messages only take `Y`/`y` and `N`/`n`, not their full-word versions (e.g. `Yes`, `no`)

## Things Other People Do

### Reproducible Builds

[Reproducible Builds recommends setting `LC_ALL`](https://reproducible-builds.org/docs/locales/).
They mention that `LC_ALL=C.UTF-8` is available everywhere.

### Nix / NixOS

Nix doesn't set any locale information; their environment is blank.
However, if you run `nix-shell -p locale --pure --run locale` everything ends up as `C` because that's the only locale available.

### Bazel

Bazel [sets `LC_ALL=C` in some test tooling](https://github.com/bazelbuild/bazel/search?q=LC_ALL) and [request tests to set a bunch of `LC_*` variables](https://docs.bazel.build/versions/main/test-encyclopedia.html#initial-conditions).

## Appendix A: Diff Between `en_US.UTF-8` and `C.UTF-8`

Output produced with `locale -ck LC_ALL` with `LC_ALL` set to either `en_US.UTF-8` or `C.UTF-8`.

```diff
--- en_US.UTF-8	2021-06-29 13:59:44.232019417 -0500
+++ C.UTF-8	2021-06-29 13:59:36.236757488 -0500
@@ -7,12 +7,12 @@
 LC_SPECIAL
 categories="LC_COLLATE LC_CTYPE LC_MESSAGES LC_MONETARY LC_NUMERIC LC_TIME"
 LC_CTYPE
-charmap="UTF-8"
+charmap="US-ASCII"
 LC_MONETARY
-currency_symbol="$"
+currency_symbol=""
 LC_TIME
-d_fmt="%m/%d/%Y"
-d_t_fmt="%a %b %e %X %Y"
+d_fmt="%m/%d/%y"
+d_t_fmt="%a %b %e %H:%M:%S %Y"
 day="Sunday";"Monday";"Tuesday";"Wednesday";"Thursday";"Friday";"Saturday"
 LC_NUMERIC
 decimal_point="."
@@ -22,41 +22,41 @@
 era_d_t_fmt=""
 era_t_fmt=""
 LC_MONETARY
-frac_digits=2
+frac_digits=127
 LC_NUMERIC
-grouping="3;3"
+grouping="0"
 LC_MONETARY
-int_curr_symbol="USD "
-int_frac_digits=2
-int_n_cs_precedes=1
-int_n_sep_by_space=0
-int_n_sign_posn=1
-int_p_cs_precedes=1
-int_p_sep_by_space=0
-int_p_sign_posn=1
+int_curr_symbol=""
+int_frac_digits=127
+int_n_cs_precedes=127
+int_n_sep_by_space=127
+int_n_sign_posn=127
+int_p_cs_precedes=127
+int_p_sep_by_space=127
+int_p_sign_posn=127
 LC_TIME
 mon="January";"February";"March";"April";"May";"June";"July";"August";"September";"October";"November";"December"
 LC_MONETARY
-mon_decimal_point="."
-mon_grouping="3;3"
-mon_thousands_sep=","
-n_cs_precedes=1
-n_sep_by_space=0
-n_sign_posn=1
-negative_sign="-"
+mon_decimal_point=""
+mon_grouping="0"
+mon_thousands_sep=""
+n_cs_precedes=127
+n_sep_by_space=127
+n_sign_posn=127
+negative_sign=""
 LC_MESSAGES
-noexpr="^[nN].*"
+noexpr="^[nN]"
 nostr="no"
 LC_MONETARY
-p_cs_precedes=1
-p_sep_by_space=0
-p_sign_posn=1
+p_cs_precedes=127
+p_sep_by_space=127
+p_sign_posn=127
 positive_sign=""
 LC_TIME
 t_fmt="%H:%M:%S"
 t_fmt_ampm="%I:%M:%S %p"
 LC_NUMERIC
-thousands_sep=","
+thousands_sep=""
 LC_MESSAGES
-yesexpr="^[yYsS].*"
+yesexpr="^[yY]"
 yesstr="yes"
```
