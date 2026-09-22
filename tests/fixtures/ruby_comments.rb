#!/usr/bin/env ruby

CONFIG_TEMPLATE = "# not a comment, just a string"
INTERPOLATED = "leading #{1 + 1} # still just a string"

# TRIPWIRE: this fixture is the ruby positive control for the gate. Delete the
# untagged comments below and a build of the checker that cannot see ruby at
# all still passes.

# CONTEXT: the =begin block below is the only multi-line comment form ruby has

=begin
TRIPWIRE: this block form carries a tag and survives
=end

=begin
an untagged block comment that the gate removes
=end

# ordinary prose that explains nothing
def f
  x = 1 # a trailing untagged comment
  x
end
