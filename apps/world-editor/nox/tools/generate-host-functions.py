import re
import sys

editor_host_path, output_path = sys.argv[1], sys.argv[2]
source = open(editor_host_path).read()

registration_pattern = re.compile(
    r'Runtime\.RegisterFunction\(\s*"(\w+)",\s*\[(\w+)\]\(([^)]*)\)\s*->\s*([\w:]+)\s*\{(.*?)\}\);', re.S)

nyx_types = {"std::string": "string", "bool": "bool", "std::int32_t": "i32", "std::int64_t": "i64",
             "float": "f32", "void": "void", "std::uint64_t": "i64"}
readers = {"std::string": "Text", "bool": "Bool", "std::int32_t": "I32", "std::int64_t": "I64", "float": "F32"}


def pascal_case(snake_name):
    return "".join(part.capitalize() for part in snake_name.split("_"))


def lower_first(name):
    return name[0].lower() + name[1:]


def translate_expression(expression, parameter_names):
    expression = expression.strip()
    expression = re.sub(r'(\w+)\.c_str\(\)', r'\1', expression)
    expression = re.sub(r'static_cast<std::int32_t>\((.*)\)$', r'i32(\1)', expression, flags=re.S)
    expression = re.sub(r'static_cast<std::int64_t>\((.*)\)$', r'i64(\1)', expression, flags=re.S)
    expression = re.sub(r'CopyString\((.*)\)$', r'\1', expression, flags=re.S)
    expression = re.sub(r'lantern_editor_(\w+)\(Editor\b', lambda match: f'Api:::{pascal_case(match.group(1))}(editor', expression)
    expression = re.sub(r'\bToIndex\(', 'HostFunctions:::ToIndex(', expression)
    expression = re.sub(r'\bLANTERN_TEMPLATE_ID\b', 'Api:::TEMPLATE_ID', expression)
    for cpp_name in parameter_names:
        expression = re.sub(rf'\b{cpp_name}\b', lower_first(cpp_name), expression)
    expression = re.sub(r'\s+', ' ', expression)
    return expression


functions = []
for match in registration_pattern.finditer(source):
    name, capture, parameter_text, return_type, body = match.groups()
    if capture != "Editor":
        continue
    parameters = []
    for parameter in [part.strip() for part in parameter_text.split(",") if part.strip()]:
        cpp_type, cpp_name = parameter.rsplit(" ", 1)
        parameters.append((cpp_type, cpp_name))
    statements = [statement.strip() for statement in body.strip().split(";") if statement.strip()]
    functions.append((name, parameters, return_type, statements))

lines = []
lines.append("Api :: import lantern:::editorapi;")
lines.append("Editor :: import lantern:::editorapi:::Editor;")
lines.append("Runtime :: import lantern:::nyx:::Runtime;")
lines.append("CallContext :: import lantern:::nyx:::CallContext;")
lines.append("NyxError :: import lantern:::nyx:::NyxError;")
lines.append("HostFunctions :: import lantern:::worldeditor:::hostfunctions;")
lines.append("")
lines.append("lantern:::worldeditor:::editorfunctions :: namespace {")
lines.append("    RegisterEditorFunctions :: (runtime: Runtime~*, state: raw Editor~*) -> NyxError*!? {")
lines.append("        failure~ NyxError*!? = null;")
for name, parameters, return_type, statements in functions:
    signature_parameters = ", ".join(f"{lower_first(cpp_name)}: {nyx_types[cpp_type]}" for cpp_type, cpp_name in parameters)
    signature = f"({signature_parameters}) -> {nyx_types[return_type]}"
    lines.append(f'        failure = HostFunctions:::FirstFailure(failure!, runtime->RegisterFunction("{name}", "{signature}", new (context: CallContext~*) -> void [state] {{')
    lines.append("            unsafe {")
    lines.append("                editor: Editor~* = state;")
    for index, (cpp_type, cpp_name) in enumerate(parameters):
        lines.append(f"                {lower_first(cpp_name)}: {nyx_types[cpp_type]}{'!' if cpp_type == 'std::string' else ''} = context->{readers[cpp_type]}({index});")
    parameter_names = [cpp_name for _, cpp_name in parameters]
    for statement in statements:
        if statement.startswith("return "):
            expression = translate_expression(statement[len("return "):], parameter_names)
            if return_type == "std::string":
                lines.append(f"                result: string! = {expression};")
                lines.append("                context->ReturnText(result);")
                lines.append("                delete result;")
            elif return_type == "bool":
                lines.append(f"                context->ReturnBool({expression});")
            elif return_type == "std::int32_t":
                lines.append(f"                context->ReturnI32(i32({expression}));")
            elif return_type in ("std::int64_t", "std::uint64_t"):
                lines.append(f"                context->ReturnI64(i64({expression}));")
            elif return_type == "float":
                lines.append(f"                context->ReturnF32({expression});")
            else:
                raise SystemExit(f"unknown return type {return_type} in {name}")
        else:
            lines.append(f"                {translate_expression(statement, parameter_names)};")
    if return_type == "void":
        lines.append("                context->ReturnNull();")
    for cpp_type, cpp_name in parameters:
        if cpp_type == "std::string":
            lines.append(f"                delete {lower_first(cpp_name)};")
    lines.append("            }")
    lines.append("        }));")
lines.append("        return failure!;")
lines.append("    }")
lines.append("")
lines.append(f"    FUNCTION_COUNT: usize : {len(functions)};")
lines.append("}")
open(output_path, "w").write("\n".join(lines) + "\n")
print(f"{len(functions)} host functions written to {output_path}")
