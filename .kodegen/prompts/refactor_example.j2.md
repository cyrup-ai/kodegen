---
title: "Refactor Code"
description: "Apply best practices and improve code quality"
categories: ["Optimize code"]
secondary_tag: "Code Analysis"
author: "kodegen-team"
verified: true
votes: 120
parameters:
  - name: "file_path"
    description: "Path to the file to refactor"
    param_type: "string"
    required: true
  - name: "language"
    description: "Programming language"
    param_type: "string"
    required: false
    default: "rust"
  - name: "focus_areas"
    description: "Specific areas to focus on"
    param_type: "array"
    required: false
    default: ["performance", "readability", "error-handling"]
---

# Refactoring {{ file_path }}

Language: **{{ language }}**

Focus areas:
{% for area in focus_areas %}
- {{ area }}
{% endfor %}

I'll analyze and refactor this {{ language }} code with focus on:
{% for area in focus_areas %}
- **{{ area | title }}**
{% endfor %}

Let me read the file and provide recommendations...
