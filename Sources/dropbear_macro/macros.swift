import SwiftCompilerPlugin
import SwiftSyntax
import SwiftSyntaxBuilder
import SwiftSyntaxMacros
import Foundation

/// Already defined in `dropbear/@ScriptEntry`.
/// A macro for a class of a script that can be used with any entity (when added).  
/// 
/// Let's say that you have an entity of a player. You want to get movement for
/// that Player. The Eucalyptus Editor only allows for one `Swift` file per entity. 
/// To combat that, there is a macro called `@ScriptEntry`, which allows for that class
/// to be ran (in no particular order) in tandem with the Player.
/// 
/// In the case that you want a script to be locked to **only** a specific entity,
/// you can use the `@Script(name: /*Entity Label*/)` to lock that class to run only on
/// that entity, improving production as you won't have to constantly rewrite scripts. 
public struct ScriptEntryMacro: MemberMacro {
    public static func expansion(
        of node: AttributeSyntax,
        providingMembersOf declaration: some DeclGroupSyntax,
        conformingTo protocols: [TypeSyntax],
        in context: some MacroExpansionContext
    ) throws -> [DeclSyntax] {
        guard declaration.is(ClassDeclSyntax.self) else {
            throw MacroError.notAClass
        }
        
        let fileName = extractFileName(from: context, node: node)
        
        return [
            """
            required init() {
                super.init()
                Task { @MainActor in
                    ScriptRegistry.registerScript(Self.self, fileName: \(literal: fileName))
                }
            }
            """
        ]
    }
}

/// Already defined in `dropbear/@Script(name: /*Entity Label*/)`.
/// A macro for a class of a script that can be used by a **specific** entity. 
/// 
/// Imagine you have an enemy. You have a class that deals with movement, a class that deals with 
/// health, but you want an Enemy specific class for its own system. This macro helps with dealing with 
/// such an issue, allowing you to attach this script to other entities as well. 
/// 
/// This macro also gives the class a higher priority compared to the `@ScriptEntry` classes, allowing this
/// script to run earlier than any ScriptEntry derived classes. 
/// 
/// FYI: This macro does not update if you change the label. If the label in editor is 
/// different than what is provided, this class will not run for that entity. 
/// 
/// # Parameters
/// - name: A String to the label of the entity set by you.  
public struct ScriptMacro: MemberMacro {
    public static func expansion(
        of node: AttributeSyntax,
        providingMembersOf declaration: some DeclGroupSyntax,
        conformingTo protocols: [TypeSyntax],
        in context: some MacroExpansionContext
    ) throws -> [DeclSyntax] {
        guard declaration.is(ClassDeclSyntax.self) else {
            throw MacroError.notAClass
        }
        
        guard case let .argumentList(args) = node.arguments,
            let firstArg = args.first,
            let labeledExpr = firstArg.expression.as(StringLiteralExprSyntax.self) else {
            throw MacroError.invalidEntityArgument
        }
        
        let entityName = labeledExpr.segments.compactMap { segment in
            if case let .stringSegment(content) = segment {
                return content.content.text
            }
            return nil
        }.joined()
        
        let fileName = extractFileName(from: context, node: node)
        
        return [
            """
            required init() {
                super.init()
                Task { @MainActor in
                    ScriptRegistry.registerScript(Self.self, fileName: \(literal: fileName))
                    ScriptRegistry.registerEntityScript(\(literal: fileName), entity: \(literal: entityName))
                }
            }
            """
        ]
    }
}

private func extractFileName(from context: some MacroExpansionContext, node: AttributeSyntax) -> String {
    if let location = context.location(of: node, at: .afterLeadingTrivia, filePathMode: .fileID) {
        let fileString = location.file.description
        let components = fileString.components(separatedBy: "/")
        if let lastComponent = components.last {
            return lastComponent.replacingOccurrences(of: ".swift", with: "")
        }
    }
    return "unknown"
}

enum MacroError: Error, CustomStringConvertible {
    case notAClass
    case invalidEntityArgument
    
    var description: String {
        switch self {
        case .notAClass:
            return "Script macros can only be applied to classes"
        case .invalidEntityArgument:
            return "Script macro requires a valid entity string argument"
        }
    }
}

@main
struct MacroPlugin: CompilerPlugin {
    let providingMacros: [Macro.Type] = [
        ScriptEntryMacro.self,
        ScriptMacro.self
    ]
}