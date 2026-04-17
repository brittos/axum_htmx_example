//! Configurações de UI centralizadas.
//!
//! Este módulo contém mapeamentos de cores e ícones para a interface de usuário.

/// Retorna a cor de fundo para uma role específica.
///
/// # Argumentos
/// * `role_name` - Nome da role (Admin, Manager, Sales, etc.)
///
/// # Retorno
/// Cor em formato hexadecimal (#RRGGBB)
pub fn role_color(role_name: &str) -> String {
    match role_name {
        "Admin" => "#3b82f6".into(),   // Azul
        "Manager" => "#10b981".into(), // Verde
        "Sales" => "#f59e0b".into(),   // Laranja
        "Viewer" => "#8b5cf6".into(),  // Roxo
        "Support" => "#06b6d4".into(), // Ciano
        _ => "#6b7280".into(),         // Cinza (padrão)
    }
}

/// Retorna o ícone Lucide para um recurso específico.
///
/// # Argumentos
/// * `resource_name` - Nome do recurso (Dashboard, User Management, etc.)
///
/// # Retorno
/// Nome do ícone Lucide (sem prefixo 'lucide-')
pub fn resource_icon(resource_name: &str) -> String {
    match resource_name {
        "Dashboard" => "layout-dashboard".into(),
        "User Management" => "users".into(),
        "Analytics" => "bar-chart-3".into(),
        "Content Library" => "library-big".into(),
        "Settings" => "settings".into(),
        "Export" => "download".into(),
        "Print" => "printer".into(),
        "Reports" => "file-text".into(),
        "Notifications" => "bell".into(),
        "Security" => "shield".into(),
        "Audit Logs" => "scroll-text".into(),
        _ => "box".into(), // Ícone padrão
    }
}

/// Retorna o ícone Lucide para uma ação específica.
///
/// # Argumentos
/// * `action_name` - Nome da ação (read, create, edit, delete, etc.)
///
/// # Retorno
/// Nome do ícone Lucide
pub fn action_icon(action_name: &str) -> String {
    match action_name {
        "read" => "eye".into(),
        "create" => "plus".into(),
        "edit" => "pencil".into(),
        "delete" => "trash-2".into(),
        "approve" => "check-circle".into(),
        "print" => "printer".into(),
        "export" => "download".into(),
        _ => "circle".into(),
    }
}

/// Cores do tema para status
pub mod status_colors {
    pub const ACTIVE: &str = "#10b981"; // Verde
    pub const INACTIVE: &str = "#6b7280"; // Cinza
    pub const PENDING: &str = "#f59e0b"; // Laranja
    pub const BANNED: &str = "#ef4444"; // Vermelho
}
