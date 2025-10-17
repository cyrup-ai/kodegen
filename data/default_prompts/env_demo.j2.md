---
title: "Environment Info"
description: "Show current environment details"
categories: ["onboarding"]
secondary_tag: "Quick Start"
author: "kodegen-team"
verified: false
votes: 0
parameters: []
---

# Your Environment

- **User:** {{ env.USER }}
- **Home:** {{ env.HOME }}
- **Shell:** {{ env.SHELL }}
- **PWD:** {{ env.PWD }}

{% if env.EDITOR %}
- **Editor:** {{ env.EDITOR }}
{% endif %}

This demonstrates how to access environment variables in prompts!

Available env vars (from [`src/tools/prompt/template.rs:91-95`](../src/tools/prompt/template.rs#L91-L95)):
- USER, HOME, SHELL, PWD, EDITOR, TERM
