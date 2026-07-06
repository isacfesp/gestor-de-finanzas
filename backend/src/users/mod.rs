// Puerta de entrada del módulo de usuarios.
//
// Este módulo NO expone rutas HTTP: en este sistema no existe registro
// público. Solo ofrece modelos y lógica interna que consumen los
// módulos auth (login, invitaciones) y admin (alta de usuarios).
pub mod models;
pub mod servicio;
