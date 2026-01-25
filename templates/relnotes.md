{%- if draft -%}
**NOTE:** Use the :pencil: links to edit those that have a `relnotes-tracking-issue`,
and they will be updated when we regenerate the notes periodically (manually).

-----

{% endif -%}
Version {{version}} ({{date}})
==========================

<a id="{{version}}-Language"></a>

Language
--------
{{language_relnotes}}

<a id="{{version}}-Compiler"></a>

Compiler
--------
{{compiler_relnotes}}

{% if platform_relnotes.len() > 0 -%}
<a id="{{version}}-Platform-Support"></a>

Platform Support
----------------
{{platform_relnotes}}

Refer to Rust's [platform support page][platform-support-doc]
for more information on Rust's tiered platform support.

[platform-support-doc]: https://doc.rust-lang.org/rustc/platform-support.html
{%- endif %}

<a id="{{version}}-Libraries"></a>

Libraries
---------
{{libraries_relnotes}}

<a id="{{version}}-Stabilized-APIs"></a>

Stabilized APIs
---------------

{{stabilized_apis_relnotes}}

These previously stable APIs are now stable in const contexts:

{{const_stabilized_apis_relnotes}}

<a id="{{version}}-Cargo"></a>

Cargo
-----
{{cargo_relnotes}}

{%- if rustdoc_relnotes.len() > 0 -%}
<a id="{{version}}-Rustdoc"></a>

Rustdoc
-----
{{rustdoc_relnotes}}

{%- endif -%}

<a id="{{version}}-Compatibility-Notes"></a>

Compatibility Notes
-------------------
{{compat_relnotes}}

<a id="{{version}}-Internal-Changes"></a>

Internal Changes
----------------

These changes do not affect any public interfaces of Rust, but they represent
significant improvements to the performance or internals of rustc and related
tools.

{{internal_changes_relnotes}}

{%- if other_relnotes.len() > 0 %}
Other
-----

{{other_relnotes}}
{%- endif -%}
