//! Utilitários de segurança.

use ammonia::clean;

/// Sanitiza uma string removendo tags HTML perigosas e scripts.
pub fn sanitize_html(input: &str) -> String {
    clean(input)
}

/// Sanitiza uma string para uso em atributos simples ou texto puro,
/// removendo qualquer tag HTML.
pub fn strip_html(input: &str) -> String {
    ammonia::Builder::empty().clean(input).to_string()
}
