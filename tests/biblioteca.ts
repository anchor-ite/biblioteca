// ============================================================
// biblioteca.test.ts — Tests unitarios del programa Biblioteca
// Ejecutar en devnet: anchor test --provider.cluster devnet
// ============================================================

import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Biblioteca } from "../target/types/biblioteca";
import { expect } from "chai";

describe("biblioteca", () => {
  const proveedor = anchor.AnchorProvider.env();
  anchor.setProvider(proveedor);

  const programa = anchor.workspace.Biblioteca as Program<Biblioteca>;
  const administrador = proveedor.wallet as anchor.Wallet;

  // PDAs que derivamos una vez para reutilizar en todos los tests
  let pdaEstado: anchor.web3.PublicKey;
  let bumpsEstado: number;

  // ISBN de prueba (13 dígitos — formato estándar ISBN-13)
  const ISBN_PRUEBA = "9786075278469";
  const ISBN_PRUEBA_2 = "9786075278470";

  let pdaLibro: anchor.web3.PublicKey;
  let pdaLibro2: anchor.web3.PublicKey;

  // --------------------------------------------------------
  // Setup: derivar PDAs antes de correr los tests
  // --------------------------------------------------------
  before(async () => {
    [pdaEstado, bumpsEstado] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("biblioteca")],
      programa.programId
    );

    [pdaLibro] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("libro"), Buffer.from(ISBN_PRUEBA)],
      programa.programId
    );

    [pdaLibro2] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("libro"), Buffer.from(ISBN_PRUEBA_2)],
      programa.programId
    );
  });

  // --------------------------------------------------------
  // 1. Inicializar biblioteca
  // --------------------------------------------------------
  it("Inicializa la biblioteca correctamente", async () => {
    await programa.methods
      .inicializar("Biblioteca UNAM — Ingeniería")
      .accounts({
        estadoBiblioteca: pdaEstado,
        administrador: administrador.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const estado = await programa.account.estadoBiblioteca.fetch(pdaEstado);

    expect(estado.nombre).to.equal("Biblioteca UNAM — Ingeniería");
    expect(estado.administrador.toBase58()).to.equal(
      administrador.publicKey.toBase58()
    );
    expect(estado.totalLibros).to.equal(0);
  });

  it("Rechaza nombre de biblioteca mayor a 50 caracteres", async () => {
    try {
      await programa.methods
        .inicializar("A".repeat(51))
        .accounts({
          estadoBiblioteca: pdaEstado,
          administrador: administrador.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
      expect.fail("Debió lanzar error");
    } catch (e: any) {
      expect(e.message).to.include("NombreDemasiado");
    }
  });

  // --------------------------------------------------------
  // 2. Registrar libros
  // --------------------------------------------------------
  it("Registra un libro nuevo correctamente", async () => {
    await programa.methods
      .registrarLibro(
        ISBN_PRUEBA,
        "Cien Años de Soledad",
        "Gabriel García Márquez",
        "863.64",  // Dewey: Literatura latinoamericana
        3          // 3 ejemplares iniciales
      )
      .accounts({
        estadoBiblioteca: pdaEstado,
        libro: pdaLibro,
        administrador: administrador.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const libro = await programa.account.libro.fetch(pdaLibro);
    const estado = await programa.account.estadoBiblioteca.fetch(pdaEstado);

    expect(libro.titulo).to.equal("Cien Años de Soledad");
    expect(libro.autor).to.equal("Gabriel García Márquez");
    expect(libro.dewey).to.equal("863.64");
    expect(libro.totalEjemplares).to.equal(3);
    expect(libro.ejemplaresDisponibles).to.equal(3);
    expect(libro.activo).to.be.true;
    expect(estado.totalLibros).to.equal(1);
  });

  it("Rechaza ISBN con longitud diferente a 13", async () => {
    try {
      await programa.methods
        .registrarLibro("123", "Título", "Autor", "800", 1)
        .accounts({
          estadoBiblioteca: pdaEstado,
          libro: pdaLibro2,
          administrador: administrador.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
      expect.fail("Debió lanzar error");
    } catch (e: any) {
      expect(e.message).to.include("IsbnInvalido");
    }
  });

  it("Rechaza registro con 0 ejemplares", async () => {
    try {
      await programa.methods
        .registrarLibro(ISBN_PRUEBA_2, "Título", "Autor", "800", 0)
        .accounts({
          estadoBiblioteca: pdaEstado,
          libro: pdaLibro2,
          administrador: administrador.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
      expect.fail("Debió lanzar error");
    } catch (e: any) {
      expect(e.message).to.include("EjemplaresCero");
    }
  });

  // --------------------------------------------------------
  // 3. Ejemplares adicionales (copias repetidas)
  // --------------------------------------------------------
  it("Actualiza ejemplares de un libro existente", async () => {
    await programa.methods
      .actualizarEjemplares(2) // Llegan 2 copias más
      .accounts({
        estadoBiblioteca: pdaEstado,
        libro: pdaLibro,
        administrador: administrador.publicKey,
      })
      .rpc();

    const libro = await programa.account.libro.fetch(pdaLibro);

    // 3 originales + 2 nuevos = 5
    expect(libro.totalEjemplares).to.equal(5);
    expect(libro.ejemplaresDisponibles).to.equal(5);
  });

  // --------------------------------------------------------
  // 4. Préstamo y devolución
  // --------------------------------------------------------
  it("Registra un préstamo correctamente", async () => {
    await programa.methods
      .prestarLibro()
      .accounts({
        estadoBiblioteca: pdaEstado,
        libro: pdaLibro,
        administrador: administrador.publicKey,
      })
      .rpc();

    const libro = await programa.account.libro.fetch(pdaLibro);
    expect(libro.ejemplaresDisponibles).to.equal(4); // 5 - 1
  });

  it("Registra devolución correctamente", async () => {
    await programa.methods
      .devolverLibro()
      .accounts({
        estadoBiblioteca: pdaEstado,
        libro: pdaLibro,
        administrador: administrador.publicKey,
      })
      .rpc();

    const libro = await programa.account.libro.fetch(pdaLibro);
    expect(libro.ejemplaresDisponibles).to.equal(5); // vuelve a 5
  });

  it("Rechaza préstamo cuando no hay ejemplares disponibles", async () => {
    // Prestar todos los ejemplares (5)
    for (let i = 0; i < 5; i++) {
      await programa.methods
        .prestarLibro()
        .accounts({
          estadoBiblioteca: pdaEstado,
          libro: pdaLibro,
          administrador: administrador.publicKey,
        })
        .rpc();
    }

    // Intentar el sexto préstamo — debe fallar
    try {
      await programa.methods
        .prestarLibro()
        .accounts({
          estadoBiblioteca: pdaEstado,
          libro: pdaLibro,
          administrador: administrador.publicKey,
        })
        .rpc();
      expect.fail("Debió lanzar error");
    } catch (e: any) {
      expect(e.message).to.include("SinEjemplaresDisponibles");
    }

    // Devolver todos para limpiar estado
    for (let i = 0; i < 5; i++) {
      await programa.methods
        .devolverLibro()
        .accounts({
          estadoBiblioteca: pdaEstado,
          libro: pdaLibro,
          administrador: administrador.publicKey,
        })
        .rpc();
    }
  });

  it("Rechaza devolución cuando todos están disponibles", async () => {
    try {
      await programa.methods
        .devolverLibro()
        .accounts({
          estadoBiblioteca: pdaEstado,
          libro: pdaLibro,
          administrador: administrador.publicKey,
        })
        .rpc();
      expect.fail("Debió lanzar error");
    } catch (e: any) {
      expect(e.message).to.include("TodosDisponibles");
    }
  });

  // --------------------------------------------------------
  // 5. Editar metadata
  // --------------------------------------------------------
  it("Edita los datos de un libro correctamente", async () => {
    await programa.methods
      .editarLibro(
        "Cien Años de Soledad (Ed. Conmemorativa)",
        "Gabriel García Márquez",
        "863.64 GAR"
      )
      .accounts({
        estadoBiblioteca: pdaEstado,
        libro: pdaLibro,
        administrador: administrador.publicKey,
      })
      .rpc();

    const libro = await programa.account.libro.fetch(pdaLibro);
    expect(libro.titulo).to.equal("Cien Años de Soledad (Ed. Conmemorativa)");
    expect(libro.dewey).to.equal("863.64 GAR");
  });

  // --------------------------------------------------------
  // 6. Dar de baja (soft delete)
  // --------------------------------------------------------
  it("Da de baja un libro sin ejemplares prestados", async () => {
    await programa.methods
      .darBajaLibro()
      .accounts({
        estadoBiblioteca: pdaEstado,
        libro: pdaLibro,
        administrador: administrador.publicKey,
      })
      .rpc();

    const libro = await programa.account.libro.fetch(pdaLibro);
    const estado = await programa.account.estadoBiblioteca.fetch(pdaEstado);

    expect(libro.activo).to.be.false;
    expect(estado.totalLibros).to.equal(0);
  });

  it("Rechaza operaciones sobre libro dado de baja", async () => {
    try {
      await programa.methods
        .prestarLibro()
        .accounts({
          estadoBiblioteca: pdaEstado,
          libro: pdaLibro,
          administrador: administrador.publicKey,
        })
        .rpc();
      expect.fail("Debió lanzar error");
    } catch (e: any) {
      expect(e.message).to.include("LibroInactivo");
    }
  });
});
