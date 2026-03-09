
// ============================================================
// biblioteca.rs — Programa de gestión de acervo bibliográfico
// Solana / Anchor 1.0.0-rc.2
// ============================================================
// Arquitectura:
//   - EstadoBiblioteca: cuenta global PDA (singleton)
//   - Libro: cuenta PDA por ISBN (seed = "libro" + isbn)
//
// Clasificación DEWEY almacenada como string (ej. "823.914")
// Ejemplares repetidos: se suma a total_ejemplares del mismo Libro
// Status derivado de ejemplares_disponibles vs total_ejemplares
// ============================================================

use anchor_lang::prelude::*;

declare_id!("45D5JdUitCeHKQ3EiDQuxrhxH5GxWPqkR7Q5W3CMaAhW");

#[program]
pub mod biblioteca {
    use super::*;

    // ----------------------------------------------------------
    // inicializar: crea el estado global de la biblioteca
    // Solo puede llamarse una vez (PDA único por programa)
    // ----------------------------------------------------------
    pub fn inicializar(ctx: Context<Inicializar>, nombre: String) -> Result<()> {
        require!(nombre.len() <= 50, ErrorBiblioteca::NombreDemasiado);

        let estado = &mut ctx.accounts.estado_biblioteca;
        estado.administrador = ctx.accounts.administrador.key();
        estado.nombre = nombre;
        estado.total_libros = 0;
        estado.bump = ctx.bumps.estado_biblioteca;

        msg!("Biblioteca '{}' inicializada", estado.nombre);
        Ok(())
    }

    // ----------------------------------------------------------
    // registrar_libro: añade un libro nuevo al acervo
    // Si el ISBN ya existe, usa actualizar_ejemplares en su lugar
    // ----------------------------------------------------------
    pub fn registrar_libro(
        ctx: Context<RegistrarLibro>,
        isbn: String,
        titulo: String,
        autor: String,
        dewey: String,       // Clasificación Dewey, ej. "823.914"
        ejemplares: u8,      // Número inicial de copias físicas
    ) -> Result<()> {
        require!(isbn.len() == 13, ErrorBiblioteca::IsbnInvalido);
        require!(titulo.len() <= 100, ErrorBiblioteca::TituloDemasiado);
        require!(autor.len() <= 80, ErrorBiblioteca::AutorDemasiado);
        require!(dewey.len() <= 20, ErrorBiblioteca::DeweyDemasiado);
        require!(ejemplares > 0, ErrorBiblioteca::EjemplaresCero);

        let libro = &mut ctx.accounts.libro;
        libro.isbn = isbn;
        libro.titulo = titulo;
        libro.autor = autor;
        libro.dewey = dewey;
        libro.total_ejemplares = ejemplares;
        libro.ejemplares_disponibles = ejemplares;
        libro.registrado_por = ctx.accounts.administrador.key();
        libro.activo = true;
        libro.bump = ctx.bumps.libro;

        // Incrementar contador global
        let estado = &mut ctx.accounts.estado_biblioteca;
        estado.total_libros = estado.total_libros.checked_add(1)
            .ok_or(ErrorBiblioteca::Overflow)?;

        msg!("Libro '{}' registrado con {} ejemplar(es)", libro.titulo, libro.total_ejemplares);
        Ok(())
    }

    // ----------------------------------------------------------
    // actualizar_ejemplares: agrega copias de un libro existente
    // Útil cuando llegan nuevas adquisiciones del mismo ISBN
    // ----------------------------------------------------------
    pub fn actualizar_ejemplares(
        ctx: Context<ActualizarLibro>,
        ejemplares_adicionales: u8,
    ) -> Result<()> {
        require!(ejemplares_adicionales > 0, ErrorBiblioteca::EjemplaresCero);

        let libro = &mut ctx.accounts.libro;
        require!(libro.activo, ErrorBiblioteca::LibroInactivo);

        libro.total_ejemplares = libro.total_ejemplares
            .checked_add(ejemplares_adicionales)
            .ok_or(ErrorBiblioteca::Overflow)?;

        libro.ejemplares_disponibles = libro.ejemplares_disponibles
            .checked_add(ejemplares_adicionales)
            .ok_or(ErrorBiblioteca::Overflow)?;

        msg!("Se añadieron {} ejemplar(es) al libro '{}'", ejemplares_adicionales, libro.titulo);
        Ok(())
    }

    // ----------------------------------------------------------
    // prestar_libro: marca un ejemplar como prestado
    // Reduce ejemplares_disponibles en 1
    // ----------------------------------------------------------
    pub fn prestar_libro(ctx: Context<ActualizarLibro>) -> Result<()> {
        let libro = &mut ctx.accounts.libro;
        require!(libro.activo, ErrorBiblioteca::LibroInactivo);
        require!(libro.ejemplares_disponibles > 0, ErrorBiblioteca::SinEjemplaresDisponibles);

        libro.ejemplares_disponibles = libro.ejemplares_disponibles
            .checked_sub(1)
            .ok_or(ErrorBiblioteca::Overflow)?;

        msg!("Préstamo registrado: '{}' — disponibles: {}/{}",
            libro.titulo, libro.ejemplares_disponibles, libro.total_ejemplares);
        Ok(())
    }

    // ----------------------------------------------------------
    // devolver_libro: registra la devolución de un ejemplar
    // Incrementa ejemplares_disponibles en 1
    // ----------------------------------------------------------
    pub fn devolver_libro(ctx: Context<ActualizarLibro>) -> Result<()> {
        let libro = &mut ctx.accounts.libro;
        require!(libro.activo, ErrorBiblioteca::LibroInactivo);
        require!(
            libro.ejemplares_disponibles < libro.total_ejemplares,
            ErrorBiblioteca::TodosDisponibles
        );

        libro.ejemplares_disponibles = libro.ejemplares_disponibles
            .checked_add(1)
            .ok_or(ErrorBiblioteca::Overflow)?;

        msg!("Devolución registrada: '{}' — disponibles: {}/{}",
            libro.titulo, libro.ejemplares_disponibles, libro.total_ejemplares);
        Ok(())
    }

    // ----------------------------------------------------------
    // editar_libro: actualiza metadata (título, autor, dewey)
    // El ISBN no se puede cambiar (es el seed del PDA)
    // ----------------------------------------------------------
    pub fn editar_libro(
        ctx: Context<ActualizarLibro>,
        nuevo_titulo: String,
        nuevo_autor: String,
        nuevo_dewey: String,
    ) -> Result<()> {
        require!(nuevo_titulo.len() <= 100, ErrorBiblioteca::TituloDemasiado);
        require!(nuevo_autor.len() <= 80, ErrorBiblioteca::AutorDemasiado);
        require!(nuevo_dewey.len() <= 20, ErrorBiblioteca::DeweyDemasiado);

        let libro = &mut ctx.accounts.libro;
        require!(libro.activo, ErrorBiblioteca::LibroInactivo);

        libro.titulo = nuevo_titulo;
        libro.autor = nuevo_autor;
        libro.dewey = nuevo_dewey;

        msg!("Datos del libro ISBN {} actualizados", libro.isbn);
        Ok(())
    }

    // ----------------------------------------------------------
    // dar_baja_libro: soft delete — marca el libro como inactivo
    // No elimina la cuenta para mantener el historial en cadena
    // ----------------------------------------------------------
    pub fn dar_baja_libro(ctx: Context<ActualizarLibro>) -> Result<()> {
        let libro = &mut ctx.accounts.libro;
        require!(libro.activo, ErrorBiblioteca::LibroInactivo);
        require!(
            libro.ejemplares_disponibles == libro.total_ejemplares,
            ErrorBiblioteca::LibrosPrestados
        );

        libro.activo = false;

        let estado = &mut ctx.accounts.estado_biblioteca;
        estado.total_libros = estado.total_libros.saturating_sub(1);

        msg!("Libro '{}' dado de baja del acervo", libro.titulo);
        Ok(())
    }
}

// ============================================================
// CUENTAS
// ============================================================

#[derive(Accounts)]
#[instruction(nombre: String)]
pub struct Inicializar<'info> {
    #[account(
        init,
        payer = administrador,
        space = EstadoBiblioteca::INIT_SPACE,
        seeds = [b"biblioteca"],
        bump
    )]
    pub estado_biblioteca: Account<'info, EstadoBiblioteca>,

    #[account(mut)]
    pub administrador: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(isbn: String)]
pub struct RegistrarLibro<'info> {
    #[account(
        mut,
        seeds = [b"biblioteca"],
        bump = estado_biblioteca.bump
    )]
    pub estado_biblioteca: Account<'info, EstadoBiblioteca>,

    #[account(
        init,
        payer = administrador,
        space = Libro::INIT_SPACE,
        // Seed compuesto: "libro" + ISBN (garantiza unicidad por ISBN)
        seeds = [b"libro", isbn.as_bytes()],
        bump
    )]
    pub libro: Account<'info, Libro>,

    #[account(
        mut,
        constraint = administrador.key() == estado_biblioteca.administrador
            @ ErrorBiblioteca::NoAutorizado
    )]
    pub administrador: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ActualizarLibro<'info> {
    #[account(
        mut,
        seeds = [b"biblioteca"],
        bump = estado_biblioteca.bump
    )]
    pub estado_biblioteca: Account<'info, EstadoBiblioteca>,

    #[account(mut)]
    pub libro: Account<'info, Libro>,

    #[account(
        mut,
        constraint = administrador.key() == estado_biblioteca.administrador
            @ ErrorBiblioteca::NoAutorizado
    )]
    pub administrador: Signer<'info>,
}

// ============================================================
// ESTADOS (datos on-chain)
// ============================================================

#[account]
#[derive(InitSpace)]
pub struct EstadoBiblioteca {
    pub administrador: Pubkey,   // Wallet con permisos de escritura
    #[max_len(50)]
    pub nombre: String,          // Nombre de la institución
    pub total_libros: u32,       // Contador de libros activos
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Libro {
    #[max_len(13)]
    pub isbn: String,            // ISBN-13 (seed del PDA)
    #[max_len(100)]
    pub titulo: String,
    #[max_len(80)]
    pub autor: String,
    #[max_len(20)]
    pub dewey: String,           // Clasificación Dewey decimal
    pub total_ejemplares: u8,    // Copias totales adquiridas
    pub ejemplares_disponibles: u8, // Copias actualmente en estante
    pub registrado_por: Pubkey,  // Wallet que registró el libro
    pub activo: bool,            // false = dado de baja (soft delete)
    pub bump: u8,
}

// Método auxiliar: el status se calcula, no se almacena
impl Libro {
    pub fn status(&self) -> &str {
        if !self.activo {
            "Dado de baja"
        } else if self.ejemplares_disponibles == 0 {
            "No disponible"
        } else {
            "Disponible"
        }
    }
}

// ============================================================
// ERRORES
// ============================================================

#[error_code]
pub enum ErrorBiblioteca {
    #[msg("No tienes permisos para realizar esta acción")]
    NoAutorizado,
    #[msg("El ISBN debe tener exactamente 13 caracteres")]
    IsbnInvalido,
    #[msg("El nombre de la biblioteca no puede exceder 50 caracteres")]
    NombreDemasiado,
    #[msg("El título no puede exceder 100 caracteres")]
    TituloDemasiado,
    #[msg("El nombre del autor no puede exceder 80 caracteres")]
    AutorDemasiado,
    #[msg("La clasificación Dewey no puede exceder 20 caracteres")]
    DeweyDemasiado,
    #[msg("Debe registrarse al menos 1 ejemplar")]
    EjemplaresCero,
    #[msg("No hay ejemplares disponibles para préstamo")]
    SinEjemplaresDisponibles,
    #[msg("Todos los ejemplares ya están disponibles, no hay devoluciones pendientes")]
    TodosDisponibles,
    #[msg("El libro ya está dado de baja")]
    LibroInactivo,
    #[msg("No se puede dar de baja un libro con ejemplares prestados")]
    LibrosPrestados,
    #[msg("Desbordamiento aritmético")]
    Overflow,
}
