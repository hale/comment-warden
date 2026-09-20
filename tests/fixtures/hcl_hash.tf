# this line explains nothing that the code doesn't already say
resource "example" "thing" {
  # TRIPWIRE: changing this endpoint mid-deploy orphans the running builder
  endpoint = "https://example.com/api" # trailing untagged comment
}
