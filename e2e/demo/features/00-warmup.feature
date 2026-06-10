Feature: Warmup

  # Throwaway scenarios. In single-worker slowMo runs, one early test slot records
  # a 0-byte video; the reporter discards anything whose slug starts with "00-warmup".

  Scenario: Warmup A
    Given I open the DOOM page

  Scenario: Warmup B
    Given I open the DOOM page
