CONFIG_TEMPLATE = """
# not a comment, just a string
"""

# TRIPWIRE: this fixture is the python positive control for the gate. Delete the
# untagged comments below and a build of the checker that cannot see python at
# all — every .py file in the repo, this checker included — still passes.

# ordinary prose that explains nothing
def f():
    x = 1  # a trailing untagged comment
    return x
